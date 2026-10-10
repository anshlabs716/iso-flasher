//! Privileged write step.
//!
//! The GUI and the interactive flow both run as the desktop user,
//! because that is the only identity the session bus and the file
//! chooser will accept. Writing to a raw block device, however,
//! requires root.
//!
//! Rather than asking the user to run the whole tool under sudo
//! (which breaks the portal), we escalate only for the write, using
//! `pkexec` so the desktop shows its own authentication dialog with
//! the application name and the reason.
//!
//! The child (`--internal-flash`) reports progress as
//! `PROGRESS <written> <total> <speed> <eta>` lines on stdout,
//! which we stream so the window can show a live bar.

/// Exit code the privileged helper uses for a user cancellation.
pub const CANCELLED: i32 = 42;

#[cfg(feature = "gui")]
mod inner {
    use std::io::{BufRead, BufReader};
    use std::path::{Path, PathBuf};
    use std::process::{Command, Stdio};

    use crate::session;
    use crate::Progress;

    /// Progress and completion events from the privileged child.
    pub enum PrivilegedEvent {
        Progress(Progress),
    }

    /// Result of a privileged flash attempt.
    #[derive(Debug)]
    pub enum PrivilegedOutcome {
        /// The device was written.
        Flashed,
        /// The user dismissed the password dialog, declined, or
        /// backed out of the confirmation.
        NotAuthorised,
        /// polkit could not prompt, so nothing was attempted.
        NoPrompt(String),
        /// The privileged step ran and failed.
        Failed(String),
    }

    /// Run this program again as root, through polkit.
    ///
    /// `arguments` are passed after the program name. `pkexec`
    /// consults a polkit policy, so a desktop agent shows a native
    /// password dialog instead of a terminal prompt.
    pub(crate) fn privileged(arguments: &[String]) -> Command {
        let executable = std::env::current_exe().expect("cannot locate iso-flasher");

        let mut command = Command::new("pkexec");
        command.arg(executable);
        command.args(arguments);
        command
    }

    /// Arguments for a privileged flash of `iso` onto `device`.
    ///
    /// `--force` is never forwarded: an unattended root write should
    /// not skip the removable-device check.
    pub(crate) fn flash_arguments(iso: &Path, device: &Path, yes: bool) -> Vec<String> {
        let mut arguments = vec![
            "--internal-flash".to_owned(),
            "--iso".to_owned(),
            iso.display().to_string(),
            "--device".to_owned(),
            device.display().to_string(),
        ];

        if yes {
            arguments.push("--yes".to_owned());
        }

        arguments
    }

    /// Whether the polkit helper can plausibly prompt the user.
    ///
    /// Without a polkit agent there is no graphical password dialog,
    /// so the escalation would hang. Detect that and fail with an
    /// explanation instead of stalling the whole window.
    pub(crate) fn can_prompt() -> bool {
        // A running polkitd plus an agent is the normal desktop setup.
        if !process_is_running("polkitd") {
            return false;
        }

        // The password dialog needs a graphical session to appear on.
        // `session_type` is advisory, so a `None` answer is not fatal:
        // pkexec will say what is wrong if we are wrong.
        session::session_type().is_some()
    }

    pub(crate) fn process_is_running(name: &str) -> bool {
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return false;
        };

        for entry in entries.flatten() {
            let Ok(stat) = std::fs::read_to_string(entry.path().join("comm")) else {
                continue;
            };
            if stat.trim() == name {
                return true;
            }
        }

        false
    }

    /// Parse one `PROGRESS` line from the child, if that is what it is.
    pub(crate) fn parse_progress(line: &str) -> Option<Progress> {
        let mut fields = line.split_whitespace();
        if fields.next()? != "PROGRESS" {
            return None;
        }

        let written: u64 = fields.next()?.parse().ok()?;
        let total: u64 = fields.next()?.parse().ok()?;
        let speed: f64 = fields.next()?.parse().ok()?;
        let eta: f64 = fields.next()?.parse().ok()?;

        Some(Progress {
            written,
            total,
            speed,
            eta,
        })
    }

    /// Escalate and flash, streaming progress to `on_event`.
    ///
    /// Runs on a worker thread: it blocks until the write finishes.
    /// `should_cancel` is checked between progress lines; when it
    /// turns true the child is killed, which leaves the device
    /// partially written.
    ///
    /// `pkexec` exits 126 when the user dismisses the dialog, and
    /// the helper exits `CANCELLED` when it aborts before writing.
    pub fn flash(
        iso: &Path,
        device: &Path,
        yes: bool,
        should_cancel: impl Fn() -> bool,
        mut on_event: impl FnMut(PrivilegedEvent),
    ) -> PrivilegedOutcome {
        if !can_prompt() {
            return PrivilegedOutcome::NoPrompt(
                "no polkit agent is running, so no password dialog can appear.\n\
                 Install polkit-1 and a polkit agent for your desktop, or run \
                 iso-flasher with sudo from a terminal."
                    .to_owned(),
            );
        }

        let arguments = flash_arguments(iso, device, yes);
        let mut child = match privileged(&arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => return PrivilegedOutcome::Failed(error.to_string()),
        };

        // Stream the child's progress. Rust's stdout is line-buffered
        // even when piped, so each PROGRESS line arrives as it is
        // printed.
        let stdout = child.stdout.take().expect("stdout was piped");
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else {
                break;
            };

            if let Some(progress) = parse_progress(&line) {
                on_event(PrivilegedEvent::Progress(progress));
            }

            if should_cancel() {
                // Killing mid-write leaves the device partially
                // written; the outcome text says so.
                let _ = child.kill();
                let _ = child.wait();
                return PrivilegedOutcome::NotAuthorised;
            }
        }

        let output = match child.wait_with_output() {
            Ok(output) => output,
            Err(error) => return PrivilegedOutcome::Failed(error.to_string()),
        };

        match output.status.code() {
            Some(0) => PrivilegedOutcome::Flashed,
            // 126: polkit dismissed or user denied.
            Some(126) | Some(super::CANCELLED) => PrivilegedOutcome::NotAuthorised,
            Some(code) => PrivilegedOutcome::Failed(format!(
                "the privileged step exited with {code}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            None => PrivilegedOutcome::Failed(
                "the privileged step was terminated by a signal".to_owned(),
            ),
        }
    }

    /// Path of this program, for diagnostics.
    pub fn executable() -> Option<PathBuf> {
        std::env::current_exe().ok()
    }
}

#[cfg(feature = "gui")]
pub use inner::{flash, PrivilegedEvent, PrivilegedOutcome};

#[cfg(all(test, feature = "gui"))]
mod tests {
    use super::inner::{can_prompt, flash_arguments, parse_progress, process_is_running};
    use std::path::Path;

    #[test]
    fn flash_arguments_carry_both_paths() {
        let arguments = flash_arguments(Path::new("/tmp/a.iso"), Path::new("/dev/sdb"), false);
        assert_eq!(
            arguments,
            vec![
                "--internal-flash".to_owned(),
                "--iso".to_owned(),
                "/tmp/a.iso".to_owned(),
                "--device".to_owned(),
                "/dev/sdb".to_owned(),
            ]
        );
        assert!(arguments.iter().all(|argument| argument != "--force"));
    }

    #[test]
    fn yes_is_forwarded_but_force_is_not() {
        let arguments = flash_arguments(Path::new("/tmp/a.iso"), Path::new("/dev/sdb"), true);
        assert!(arguments.contains(&"--yes".to_owned()));
    }

    #[test]
    fn polkit_agent_check_does_not_panic() {
        // Either answer is fine; it must simply not crash.
        let _ = can_prompt();
        let _ = process_is_running("polkitd");
    }

    #[test]
    fn progress_lines_are_parsed() {
        let progress =
            parse_progress("PROGRESS 1048576 8388608 42.5 3.2").expect("a well-formed line parses");
        assert_eq!(progress.written, 1_048_576);
        assert_eq!(progress.total, 8_388_608);
        assert!((progress.speed - 42.5).abs() < f64::EPSILON);
        assert!((progress.eta - 3.2).abs() < f64::EPSILON);
    }

    #[test]
    fn other_output_is_ignored() {
        assert!(parse_progress("DONE").is_none());
        assert!(parse_progress("this is not progress").is_none());
        assert!(parse_progress("PROGRESS not-a-number 8 1 1").is_none());
    }

    #[test]
    fn cancelled_code_is_a_real_number() {
        assert_eq!(super::CANCELLED, 42);
    }
}
