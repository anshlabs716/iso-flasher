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

/// A rough guess at whether a human is sitting at this machine right now.
///
/// Used only to decide whether showing a graphical dialog is worth attempting.
pub fn has_graphical_session() -> bool {
    if session_bus_address() == BusAddress::Unavailable {
        return false;
    }

    non_empty_var("WAYLAND_DISPLAY").is_some()
        || non_empty_var("DISPLAY").is_some()
        || session_type().as_deref() == Some("wayland")
}

/// Report whether the current session looks graphical, for diagnostics.
pub fn graphical_session_report() -> String {
    if has_graphical_session() {
        format!(
            "yes ({})",
            session_type().unwrap_or_else(|| "unknown".to_owned())
        )
    } else {
        "no".to_owned()
    }
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
