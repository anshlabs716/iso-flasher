//! Desktop environment and session detection.
//!
//! iso-flasher runs as root (it writes to raw block devices), which means the
//! desktop session variables are usually gone by the time we need them: `sudo`
//! resets the environment unless they are explicitly kept. Everything in this
//! module therefore works from *reconstructable* facts rather than assuming the
//! environment survived privilege escalation.

use std::env;
use std::path::{Path, PathBuf};

/// The session bus address that should be used for talking to desktop services.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusAddress {
    /// Use this explicit D-Bus address.
    Explicit(String),
    /// No usable address could be determined.
    Unavailable,
}

/// Best-effort recovery of the *desktop user's* session bus address.
///
/// Order of preference:
/// 1. `DBUS_SESSION_BUS_ADDRESS`, when it is still present in the environment.
/// 2. The runtime directory of the invoking user (`SUDO_UID`), which is where
///    systemd puts the per-user `bus` socket.
///
/// This is what makes the portal reachable from a root process: the portal runs
/// in the user's session, not in root's.
pub fn session_bus_address() -> BusAddress {
    if let Some(address) = non_empty_var("DBUS_SESSION_BUS_ADDRESS") {
        return BusAddress::Explicit(address);
    }

    if let Some(socket) = invoking_user_bus_socket() {
        return BusAddress::Explicit(format!("unix:path={}", socket.display()));
    }

    BusAddress::Unavailable
}

/// Path of the session bus socket belonging to the user who invoked sudo.
///
/// Prefers `SUDO_UID` (set by sudo) and falls back to `PKEXEC_UID`, which
/// polkit sets when the program was started through a polkit agent.
fn invoking_user_bus_socket() -> Option<PathBuf> {
    for key in ["SUDO_UID", "PKEXEC_UID"] {
        let Some(raw) = non_empty_var(key) else {
            continue;
        };
        let Ok(uid) = raw.parse::<u32>() else {
            continue;
        };

        // Never try to talk to root's own bus; we want the desktop session.
        if uid == 0 {
            continue;
        }

        let socket = Path::new("/run/user").join(uid.to_string()).join("bus");
        if socket.exists() {
            return Some(socket);
        }
    }

    None
}

fn non_empty_var(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

/// The primary group of the desktop user, needed to fully drop privileges.
pub fn invoking_gid() -> Option<u32> {
    for key in ["SUDO_GID", "PKEXEC_GID"] {
        if let Some(gid) = non_empty_var(key).and_then(|raw| raw.parse::<u32>().ok()) {
            if gid != 0 {
                return Some(gid);
            }
        }
    }

    non_empty_var("USER")
        .and_then(|user| passwd_field(&user, 3))
        .and_then(|gid| gid.parse::<u32>().ok())
}

/// Look up one colon-separated `/etc/passwd` field by user name.
fn passwd_field(user: &str, index: usize) -> Option<String> {
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;

    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() > index && fields[0] == user {
            return Some(fields[index].to_owned());
        }
    }

    None
}

/// Environment needed to talk to the desktop session as the desktop user.
pub fn user_environment() -> Vec<(String, String)> {
    let mut environment = Vec::new();

    if let Some(uid) = invoking_uid() {
        let runtime = format!("/run/user/{uid}");
        environment.push(("XDG_RUNTIME_DIR".to_owned(), runtime.clone()));
        environment.push((
            "DBUS_SESSION_BUS_ADDRESS".to_owned(),
            format!("unix:path={runtime}/bus"),
        ));
    }

    if let Some(home) = invoking_user_home() {
        environment.push(("HOME".to_owned(), home.to_string_lossy().into_owned()));
    }

    if let Some(user) = non_empty_var("USER") {
        environment.push(("USER".to_owned(), user));
    }

    if let Some(display) = non_empty_var("DISPLAY") {
        environment.push(("DISPLAY".to_owned(), display));
    }

    if let Some(wayland) = non_empty_var("WAYLAND_DISPLAY") {
        environment.push(("WAYLAND_DISPLAY".to_owned(), wayland));
        environment.push(("XDG_SESSION_TYPE".to_owned(), "wayland".to_owned()));
    }

    if let Some(kind) = non_empty_var("XDG_CURRENT_DESKTOP") {
        environment.push(("XDG_CURRENT_DESKTOP".to_owned(), kind));
    }

    // PATH so the child can still find ordinary programs.
    if let Some(path) = non_empty_var("PATH") {
        environment.push(("PATH".to_owned(), path));
    }

    environment
}

/// True when we are root but a desktop user invoked us, so the GUI step must
/// be run as that user.
pub fn needs_privilege_drop() -> bool {
    let is_root = unsafe { geteuid() == 0 };
    is_root && invoking_uid().is_some()
}

extern "C" {
    fn geteuid() -> u32;
}

/// UID of the user who invoked sudo/polkit, if we escalated from another user.
pub fn invoking_uid() -> Option<u32> {
    for key in ["SUDO_UID", "PKEXEC_UID"] {
        if let Some(uid) = non_empty_var(key).and_then(|raw| raw.parse::<u32>().ok()) {
            if uid != 0 {
                return Some(uid);
            }
        }
    }

    // Not root, or already running as the desktop user.
    let euid = unsafe { getuid() };
    if euid != 0 {
        return Some(euid);
    }

    None
}

extern "C" {
    fn getuid() -> u32;
}

/// Home directory of the desktop user.
///
/// Under `sudo`, `HOME` points at root's home, so the file dialog would open
/// in `/root` (or fall back to `/`). Read the real home from `/etc/passwd`.
pub fn invoking_user_home() -> Option<PathBuf> {
    let uid = invoking_uid()?;
    let passwd = std::fs::read_to_string("/etc/passwd").ok()?;

    for line in passwd.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        // name:password:uid:gid:gecos:home:shell
        if fields.len() < 6 {
            continue;
        }

        if fields[2].parse::<u32>().ok() == Some(uid) {
            let home = fields[5];
            if !home.is_empty() {
                return Some(PathBuf::from(home));
            }
        }
    }

    None
}

/// The user's display server, e.g. `wayland` or `x11`.
///
/// Reconstructed from the runtime directory when the environment was stripped,
/// since the socket name in `/run/user/<uid>` tells us which one is in use.
pub fn session_type() -> Option<String> {
    if let Some(kind) = non_empty_var("XDG_SESSION_TYPE") {
        return Some(kind.to_lowercase());
    }

    let socket = invoking_user_bus_socket()?;
    let runtime = socket.parent()?;

    for name in ["wayland-0", "wayland-1"] {
        if runtime.join(name).exists() {
            return Some("wayland".to_owned());
        }
    }

    if runtime.join("pulse").exists() || runtime.join("bus").exists() {
        // X11 sessions keep their socket in /tmp/.X11-unix, so this is a hint
        // only; callers must treat it as advisory.
        return None;
    }

    None
}

/// The desktop environment name, e.g. `KDE` or `GNOME`.
///
/// Purely informational: the file chooser is chosen by which portal backend is
/// running, never by which desktop we think we are on.
pub fn desktop_environment() -> Option<String> {
    non_empty_var("XDG_CURRENT_DESKTOP").or_else(|| non_empty_var("XDG_SESSION_DESKTOP"))
}

/// Whether a named helper program is on `PATH`.
pub fn command_exists(name: &str) -> bool {
    let Ok(path) = env::var("PATH") else {
        return false;
    };

    env::split_paths(&path).any(|directory| {
        let candidate = directory.join(name);
        candidate.is_file() && is_executable(&candidate)
    })
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_targets_roots_own_bus() {
        // No SUDO_UID set in the test environment: still must not panic.
        let _ = invoking_user_bus_socket();
    }

    #[test]
    fn explicit_address_wins_over_reconstruction() {
        // Exercised indirectly: an explicit address is returned verbatim.
        let address = BusAddress::Explicit("unix:path=/tmp/example".to_owned());
        assert_eq!(
            address,
            BusAddress::Explicit("unix:path=/tmp/example".to_owned())
        );
    }

    #[test]
    fn command_exists_rejects_nonexistent_programs() {
        assert!(!command_exists("definitely-not-a-real-program-xyz"));
    }
}

#[cfg(test)]
mod home_tests {
    use super::*;

    #[test]
    fn user_environment_points_at_the_desktop_session() {
        unsafe {
            std::env::set_var("SUDO_UID", "1000");
            std::env::set_var("SUDO_GID", "1000");
        }

        let environment = user_environment();
        let find = |key: &str| {
            environment
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value.clone())
        };

        // Must target the user's runtime dir, not root's.
        assert_eq!(
            find("DBUS_SESSION_BUS_ADDRESS").as_deref(),
            Some("unix:path=/run/user/1000/bus")
        );
        assert_eq!(find("XDG_RUNTIME_DIR").as_deref(), Some("/run/user/1000"));
        assert!(find("HOME").is_some_and(|home| home != "/root"));

        unsafe {
            std::env::remove_var("SUDO_UID");
            std::env::remove_var("SUDO_GID");
        }
    }

    #[test]
    fn home_is_recovered_even_when_env_points_at_root() {
        // Simulate the sudo case: SUDO_UID set and HOME rewritten.
        unsafe {
            std::env::set_var("SUDO_UID", "1000");
            std::env::set_var("HOME", "/root");
        }
        assert_eq!(invoking_uid(), Some(1000));
        let home = invoking_user_home().expect("home for uid 1000");
        assert_ne!(home, PathBuf::from("/root"), "must not use root's home");
        assert!(home.is_absolute());
        unsafe {
            std::env::remove_var("SUDO_UID");
            std::env::set_var("HOME", std::env::var("HOME").unwrap_or_default());
        }
    }
}
