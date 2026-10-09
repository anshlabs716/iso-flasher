//! Removable block device discovery and validation.
//!
//! A USB stick is a raw block device, not a file, so it will never show up in a
//! desktop file chooser and must never be looked for there. We enumerate real
//! block devices from sysfs instead, describe them well enough that a user can
//! recognise the right one, and validate that whatever was chosen is a whole
//! device rather than one of its partitions.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SYS_BLOCK: &str = "/sys/block";
const SYS_CLASS_BLOCK: &str = "/sys/class/block";

/// Loop/ram/dm/md style nodes that are never flash targets.
const PSEUDO_PREFIXES: &[&str] = &["loop", "ram", "zram", "dm-", "md", "sr", "fd", "nbd"];

/// A whole block device that is a plausible flashing target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockDevice {
    pub path: PathBuf,
    pub name: String,
    pub size_bytes: u64,
    pub model: Option<String>,
    pub vendor: Option<String>,
    pub serial: Option<String>,
    pub removable: bool,
    /// e.g. `usb`, `nvme`, `sata`, `mmc`
    pub transport: Option<String>,
}

impl BlockDevice {
    /// Human-readable one-line summary, e.g. `8.0 GB  Sanitizer  (MXT-USB)`.
    pub fn describe(&self) -> String {
        let mut parts = vec![format_bytes(self.size_bytes)];
        if let Some(model) = &self.model {
            parts.push(model.clone());
        }
        if let Some(vendor) = &self.vendor {
            parts.push(format!("({vendor})"));
        }
        if let Some(transport) = &self.transport {
            parts.push(format!("[{transport}]"));
        }
        parts.join("  ")
    }
}

/// Format a byte count using binary units.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }

    format!("{value:.1} {}", UNITS[unit])
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// sysfs reports disk size in 512-byte sectors regardless of block size.
fn size_from_sysfs(sys_path: &Path) -> Option<u64> {
    let sectors: u64 = read_trimmed(&sys_path.join("size"))?.parse().ok()?;
    sectors.checked_mul(512)
}

fn attribute(device: &PathSysPath, name: &str) -> Option<String> {
    read_trimmed(&device.device_dir().join(name))
        .map(|value| value.replace('\t', " "))
        .map(|value| value.trim().to_owned())
}

/// Small helper so `attribute` can find the USB device node for a disk.
struct PathSysPath<'a> {
    sys_path: &'a Path,
}

impl<'a> PathSysPath<'a> {
    fn device_dir(&self) -> PathBuf {
        // /sys/block/<name>/device points at the parent device (e.g. the USB
        // device node) for USB-attached disks, and is absent for some virtio
        // devices. Attribute lookups tolerate its absence.
        self.sys_path.join("device")
    }
}

/// Identify the transport by walking the sysfs device path.
fn detect_transport(sys_path: &Path) -> Option<String> {
    let resolved = fs::canonicalize(sys_path.join("device")).ok()?;
    let text = resolved.to_string_lossy();

    for (needle, name) in [
        ("/usb", "usb"),
        ("/nvme/", "nvme"),
        ("/mmc", "mmc"),
        ("/ata", "sata"),
        ("/scsi", "scsi"),
        ("/virtio", "virtio"),
        ("/virtio_blk", "virtio"),
    ] {
        if text.contains(needle) {
            return Some(name.to_owned());
        }
    }

    None
}

fn is_pseudo(name: &str) -> bool {
    PSEUDO_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Build a [`BlockDevice`] for one `/sys/block/<name>` entry.
fn describe_sys_device(name: &str, sys_path: &Path) -> Option<BlockDevice> {
    if is_pseudo(name) {
        return None;
    }

    let path = Path::new("/dev").join(name);
    let size_bytes = size_from_sysfs(sys_path)?;
    if size_bytes == 0 {
        return None;
    }

    let removable = read_trimmed(&sys_path.join("removable")).as_deref() == Some("1");
    let device = PathSysPath { sys_path };

    Some(BlockDevice {
        path,
        name: name.to_owned(),
        size_bytes,
        model: attribute(&device, "model"),
        vendor: attribute(&device, "vendor"),
        serial: attribute(&device, "serial"),
        removable,
        transport: detect_transport(sys_path),
    })
}

/// All whole block devices the kernel currently exposes.
pub fn all_devices() -> Vec<BlockDevice> {
    let Ok(entries) = fs::read_dir(SYS_BLOCK) else {
        return Vec::new();
    };

    let mut devices: Vec<BlockDevice> = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            describe_sys_device(&name, &entry.path())
        })
        .collect();

    devices.sort_by(|left, right| left.name.cmp(&right.name));
    devices
}

/// Devices that look like USB sticks: flagged removable, or on a USB bus.
///
/// A USB stick plugged into a hub sometimes reports `removable=0`, so transport
/// is treated as equally strong a signal.
pub fn removable_devices() -> Vec<BlockDevice> {
    all_devices()
        .into_iter()
        .filter(|device| device.removable || device.transport.as_deref() == Some("usb"))
        .collect()
}

/// Whether a path names a partition rather than a whole device.
///
/// This is the check that stops `/dev/sdb1` from being accepted.
pub fn is_partition(device: &Path) -> bool {
    let Some(name) = device.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    Path::new(SYS_CLASS_BLOCK)
        .join(name)
        .join("partition")
        .exists()
}

/// Verify that `path` is a whole block device we are willing to write to.
///
/// Rejects anything that is not a direct child of `/dev`, any partition, and
/// any node that is not a real block device.
pub fn validate_target(path: &Path) -> io::Result<BlockDevice> {
    let parent = path.parent();
    if parent != Some(Path::new("/dev")) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a device directly under /dev", path.display()),
        ));
    }

    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid device name"))?;

    if is_partition(path) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("/dev/{name} is a partition; select the whole device (for example /dev/sdb) instead"),
        ));
    }

    let sys_path = Path::new(SYS_BLOCK).join(name);
    let Some(device) = describe_sys_device(name, &sys_path) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("/dev/{name} is not a whole block device"),
        ));
    };

    Ok(device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes_readably() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GiB");
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn skips_pseudo_devices() {
        assert!(is_pseudo("loop0"));
        assert!(is_pseudo("zram0"));
        assert!(is_pseudo("dm-0"));
        assert!(!is_pseudo("sda"));
        assert!(!is_pseudo("nvme0n1"));
    }

    #[test]
    fn enumeration_does_not_panic_without_sysfs() {
        // On a normal Linux host this returns real devices; elsewhere empty.
        for device in all_devices() {
            assert!(device.path.starts_with("/dev/"));
            assert!(!is_pseudo(&device.name));
        }
    }

    #[test]
    fn partitions_are_rejected_by_validation() {
        // /dev/sda1 is a real partition on this host.
        if Path::new("/dev/sda1").exists() {
            let error = validate_target(Path::new("/dev/sda1")).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert!(error.to_string().contains("partition"));
        }
    }

    #[test]
    fn whole_device_is_accepted() {
        if Path::new("/dev/sda").exists() {
            let device = validate_target(Path::new("/dev/sda")).expect("sda is a whole device");
            assert_eq!(device.name, "sda");
            assert!(device.size_bytes > 0);
        }
    }

    #[test]
    fn paths_outside_dev_are_rejected() {
        let error = validate_target(Path::new("/tmp/not-a-device")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn missing_nodes_are_rejected() {
        let error = validate_target(Path::new("/dev/definitely-not-here")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
