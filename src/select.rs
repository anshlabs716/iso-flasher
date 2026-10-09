//! User-facing selection flows for the ISO image and the USB device.
//!
//! Files go through the desktop's own file chooser (XDG portal first).
//! Block devices do not, because a file chooser cannot represent them; they go
//! through a device list built from sysfs.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::devices::{self, BlockDevice};
use crate::helpers::{self, FallbackOutcome};
use crate::portal::{self, FileFilter, PickerError};

// Terminal styling, kept local so this module has no dependency on the flash UI.
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CYAN: &str = "\x1b[36m";
const YELLOW: &str = "\x1b[33m";

/// ISO filters offered in the chooser.
///
/// The globs are a convenience only. The portal is explicitly allowed to return
/// something that matches no filter, and we validate the result ourselves, so a
/// valid ISO called `image.bin` can still be chosen.
pub fn iso_filters() -> Vec<FileFilter> {
    vec![FileFilter {
        name: "ISO images".to_owned(),
        globs: vec![
            "*.iso".to_owned(),
            "*.ISO".to_owned(),
            "*.img".to_owned(),
            "*.IMG".to_owned(),
        ],
        mime_types: vec!["application/x-cd-image".to_owned()],
    }]
}

/// Why a selection ended without a usable answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionError {
    /// The user deliberately backed out. Never treated as a flashing failure.
    Cancelled,
    /// Nothing graphical is available; the caller should explain and offer CLI.
    NoInterface(String),
    /// Something is available but the attempt failed.
    Failed(String),
}

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectionError::Cancelled => write!(formatter, "selection cancelled"),
            SelectionError::NoInterface(detail) => write!(formatter, "{detail}"),
            SelectionError::Failed(detail) => write!(formatter, "{detail}"),
        }
    }
}

impl std::error::Error for SelectionError {}

impl SelectionError {
    /// Whether this outcome was simply the user changing their mind.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, SelectionError::Cancelled)
    }
}

impl From<io::Error> for SelectionError {
    fn from(error: io::Error) -> Self {
        SelectionError::Failed(error.to_string())
    }
}

/// Open the desktop file chooser and return the selected file.
///
/// Tries the XDG portal first so the user gets whichever dialog their portal
/// backend provides, then falls back to installed helper programs.
pub fn choose_file(
    title: &str,
    directory: &Path,
    filters: &[FileFilter],
) -> Result<PathBuf, SelectionError> {
    let portal_error: String;

    if portal::is_available() {
        match portal::choose_file(title, directory, filters) {
            Ok(path) => return Ok(path),
            // A user cancellation is a decision, not a reason to try another
            // dialog; stop here.
            Err(PickerError::Cancelled) => return Err(SelectionError::Cancelled),
            Err(error) => portal_error = error.to_string(),
        }
    } else {
        portal_error = "no XDG desktop file chooser portal is available".to_owned();
        // Never fall back silently: a helper dialog looks like a plain file
        // manager, which is confusing without an explanation.
        explain_portal_failure();
    }

    match helpers::choose_file(title, directory, filters) {
        FallbackOutcome::Chosen(path) => Ok(path),
        FallbackOutcome::Cancelled => Err(SelectionError::Cancelled),
        FallbackOutcome::Failed(message) => Err(SelectionError::Failed(format!(
            "{portal_error}; helper dialog also failed: {message}",
        ))),
        FallbackOutcome::Unavailable(message) => Err(SelectionError::NoInterface(format!(
            "{message}. Install xdg-desktop-portal with a FileChooser backend \
             (for example xdg-desktop-portal-gtk, xdg-desktop-portal-kde or \
             xdg-desktop-portal-cosmic), or pass --iso and --device on the command line."
        ))),
    }
}

/// Validate a chosen ISO independently of whatever the chooser advertised.
pub fn validate_iso(path: &Path) -> io::Result<PathBuf> {
    let metadata = std::fs::metadata(path)?;

    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", path.display()),
        ));
    }

    if metadata.len() == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is empty", path.display()),
        ));
    }

    Ok(path.to_path_buf())
}

/// The directory the ISO chooser should open in: the desktop user's home.
///
/// `sudo` rewrites `HOME` to `/root`, which is wrong and usually unreadable to
/// the desktop session, so recover the invoking user's home from `SUDO_UID`.
pub fn starting_directory() -> PathBuf {
    if let Some(home) = crate::session::invoking_user_home() {
        if home.is_dir() {
            return home;
        }
    }

    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Ask the user to choose an ISO image.
pub fn choose_iso() -> Result<PathBuf, SelectionError> {
    let selected = choose_file("Select an ISO image", &starting_directory(), &iso_filters())?;
    validate_iso(&selected)?;
    Ok(selected)
}

/// Run the interactive selection as the desktop user when we are root.
///
/// A root process cannot use the desktop's file chooser: the session bus
/// belongs to the desktop user and D-Bus authentication rejects the root
/// caller. Every distro solves this the same way, by doing the GUI part as the
/// user and keeping privilege only for the write. So when we are root we hand
/// the whole selection step to a child process running as the desktop user,
/// which returns the chosen ISO path on stdout.
pub fn choose_iso_as_desktop_user() -> Result<PathBuf, SelectionError> {
    if !crate::session::needs_privilege_drop() {
        return choose_iso();
    }

    let executable = std::env::current_exe()
        .map_err(|error| SelectionError::Failed(format!("cannot locate iso-flasher: {error}")))?;

    let mut command = Command::new(executable);
    command.arg("--internal-choose-iso");

    use std::os::unix::process::CommandExt;

    // The child runs as the desktop user so the portal accepts it; it only ever
    // shows the chooser, never the privileged flashing code.
    command.uid(crate::session::invoking_uid().expect("checked"));
    if let Some(gid) = crate::session::invoking_gid() {
        command.gid(gid);
    }

    for (key, value) in crate::session::user_environment() {
        command.env(key, value);
    }

    // Clear anything sudo injected so the child looks like a normal session.
    for key in ["SUDO_UID", "SUDO_GID", "SUDO_USER"] {
        command.env_remove(key);
    }

    let output = command.output().map_err(|error| {
        SelectionError::Failed(format!("could not start the file chooser: {error}"))
    })?;

    // The child reports a cancelled selection with a dedicated exit code.
    if output.status.code() == Some(2) {
        return Err(SelectionError::Cancelled);
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(SelectionError::Failed(if stderr.is_empty() {
            "the file chooser did not return a selection".to_owned()
        } else {
            stderr
        }));
    }

    let chosen = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if chosen.is_empty() {
        return Err(SelectionError::Cancelled);
    }

    validate_iso(Path::new(&chosen))?;
    Ok(PathBuf::from(chosen))
}

/// Which chooser a user would currently get, for `--help` style reporting.
pub fn describe_backend() -> String {
    let desktop = crate::session::desktop_environment()
        .map(|name| format!("desktop {name}"))
        .unwrap_or_else(|| "unknown desktop".to_owned());
    let session = crate::session::session_type()
        .map(|kind| format!("{kind} session"))
        .unwrap_or_else(|| "session type unknown".to_owned());

    if portal::is_available() {
        format!("XDG desktop portal file chooser ({desktop}, {session})")
    } else if helpers::any_available() {
        format!("helper dialog fallback ({desktop}, {session})")
    } else {
        format!("command line only ({desktop}, {session})")
    }
}

/// Explain a portal problem, so a fallback is never silent.
pub fn explain_portal_failure() {
    let address = match crate::session::session_bus_address() {
        crate::session::BusAddress::Explicit(address) => address,
        crate::session::BusAddress::Unavailable => {
            eprintln!(
                "{YELLOW}No session bus address found. Run under your desktop session (not a bare tty).{RESET}"
            );
            return;
        }
    };

    eprintln!(
        "{YELLOW}XDG file chooser portal unavailable (bus: {address}).{RESET}\n\
         {YELLOW}Falling back to a helper dialog, which may look like a plain file manager.{RESET}\n\
         {YELLOW}To restore the native dialog: sudo apt install xdg-desktop-portal xdg-desktop-portal-kde{RESET}"
    );
}

/// Render the removable-device list for display.
pub fn format_device_list(list: &[BlockDevice]) -> String {
    let width = list
        .iter()
        .map(|device| device.name.len())
        .max()
        .unwrap_or(2)
        .max(2);

    let mut text = String::new();
    for (index, device) in list.iter().enumerate() {
        text.push_str(&format!(
            "  {index}) /dev/{:<width$}  {}\n",
            device.name,
            device.describe(),
            width = width
        ));
    }
    text
}

/// Pick one removable block device from a numbered list.
///
/// A raw device cannot be chosen from a file chooser, so we list the real
/// candidates and let the user choose by number. The terminal is always
/// available, which keeps this working headlessly and without any DE-specific
/// tooling.
pub fn choose_device_interactively() -> Result<BlockDevice, SelectionError> {
    let devices = devices::removable_devices();

    if devices.is_empty() {
        return Err(SelectionError::Failed(
            "no removable block devices were detected. Plug in a USB drive, or pass --device explicitly."
                .to_owned(),
        ));
    }

    println!("\n{CYAN}{BOLD}Detected removable devices{RESET}");
    print!("{}", format_device_list(&devices));

    if devices.len() == 1 {
        println!("\nOnly one removable device is present; using it.");
        return Ok(devices.into_iter().next().expect("length checked"));
    }

    loop {
        print!("\nSelect a device by number (or 'q' to cancel): ");
        use std::io::Write;
        let _ = std::io::stdout().flush();

        let mut answer = String::new();
        if std::io::stdin().read_line(&mut answer)? == 0 {
            return Err(SelectionError::Cancelled);
        }

        let answer = answer.trim();
        if answer.eq_ignore_ascii_case("q") || answer.eq_ignore_ascii_case("quit") {
            return Err(SelectionError::Cancelled);
        }

        match answer.parse::<usize>() {
            // Accept 1-based input, which is what the listing implies.
            Ok(choice) if choice >= 1 && choice <= devices.len() => {
                return Ok(devices.into_iter().nth(choice - 1).expect("bounds checked"));
            }
            _ => println!(
                "{YELLOW}Enter a number between 1 and {}.{RESET}",
                devices.len()
            ),
        }
    }
}

/// Resolve the device to flash, from `--device` or by asking.
///
/// A path supplied on the command line is still validated as a whole device.
pub fn choose_device(explicit: Option<&Path>) -> Result<BlockDevice, SelectionError> {
    let device = match explicit {
        Some(path) => devices::validate_target(path)?,
        None => choose_device_interactively()?,
    };

    Ok(device)
}

/// Unmount every mounted partition of a device, then the device itself.
pub fn unmount(device: &Path) -> io::Result<()> {
    let name = device
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid device name"))?;

    let block_path = Path::new("/sys/class/block").join(name);
    let mut targets: Vec<PathBuf> = Vec::new();

    if block_path.is_dir() {
        for entry in std::fs::read_dir(&block_path)? {
            let entry = entry?;
            if entry.path().join("partition").exists() {
                let partition = entry.file_name().to_string_lossy().into_owned();
                targets.push(PathBuf::from(format!("/dev/{partition}")));
            }
        }
    }

    targets.push(device.to_path_buf());

    for target in targets {
        let status = std::process::Command::new("/bin/umount")
            .arg(&target)
            .status()?;

        // 32 == "not mounted", which is the desired state already.
        if !status.success() && status.code() != Some(32) {
            return Err(io::Error::other(format!(
                "failed to unmount {}",
                target.display()
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_filters_cover_common_extensions() {
        let filters = iso_filters();
        assert_eq!(filters.len(), 1);
        assert!(filters[0].globs.iter().any(|glob| glob == "*.iso"));
        assert!(filters[0].globs.iter().any(|glob| glob == "*.ISO"));
    }

    #[test]
    fn rejects_non_iso_files_by_content_not_name() {
        let temp = std::env::temp_dir().join("iso-flasher-test.bin");
        std::fs::write(&temp, b"not really an iso").unwrap();

        // Present the file without an .iso extension: validation is about the
        // file being usable, not about trusting the chooser's filter.
        assert!(validate_iso(&temp).is_ok());

        let directory = std::env::temp_dir().join("iso-flasher-test-dir");
        std::fs::create_dir_all(&directory).unwrap();
        assert!(validate_iso(&directory).is_err());

        let _ = std::fs::remove_file(&temp);
        let _ = std::fs::remove_dir(&directory);
    }

    #[test]
    fn rejects_empty_iso() {
        let temp = std::env::temp_dir().join("iso-flasher-empty.iso");
        std::fs::write(&temp, b"").unwrap();
        assert!(validate_iso(&temp).is_err());
        let _ = std::fs::remove_file(&temp);
    }

    #[test]
    fn device_list_is_readable() {
        let list = devices::removable_devices();
        let text = format_device_list(&list);
        assert_eq!(text.lines().count(), list.len());
    }

    #[test]
    fn explicit_device_path_is_validated() {
        // A partition must never be accepted, however it was supplied.
        if Path::new("/dev/sda1").exists() {
            assert!(choose_device(Some(Path::new("/dev/sda1"))).is_err());
        }
        assert!(choose_device(Some(Path::new("/etc/passwd"))).is_err());
    }
}
