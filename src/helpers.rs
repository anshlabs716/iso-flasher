//! Optional command-line dialog helpers.
//!
//! These are *fallbacks* for the rare case where no FileChooser portal is
//! available (a minimal window manager, a container, a stripped-down install).
//! The portal is always tried first. None of these tools is required, and none
//! of them is specific to a particular file manager.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::portal::FileFilter;
use crate::session::command_exists;

/// Outcome of a helper-driven chooser.
pub enum FallbackOutcome {
    Chosen(PathBuf),
    Cancelled,
    Unavailable(String),
    Failed(String),
}

/// True if any graphical chooser helper is installed.
pub fn any_available() -> bool {
    helpers().into_iter().next().is_some()
}

/// Installed helper programs, in the order we prefer to use them.
fn helpers() -> Vec<&'static str> {
    ["zenity", "kdialog", "yad"]
        .into_iter()
        .filter(|name| command_exists(name))
        .collect()
}

/// Show a file chooser using whichever helper is installed.
pub fn choose_file(title: &str, directory: &Path, filters: &[FileFilter]) -> FallbackOutcome {
    let Some(helper) = helpers().into_iter().next() else {
        return FallbackOutcome::Unavailable(
            "no XDG file chooser portal and no kdialog/zenity/yad helper found".to_owned(),
        );
    };

    let output = match helper {
        "zenity" => zenity(helper, title, directory, filters),
        _ => kdialog(helper, title, directory, filters),
    };

    match output {
        Ok(selection) => match selection {
            Some(path) => FallbackOutcome::Chosen(path),
            None => FallbackOutcome::Cancelled,
        },
        Err(message) => FallbackOutcome::Failed(message),
    }
}

/// Run a helper and interpret its exit status.
///
/// These dialogs follow the usual convention: exit 0 with a path on stdout
/// means chosen, exit 1 means the user cancelled, anything else is a real error.
fn run(command: &mut Command) -> Result<Option<PathBuf>, String> {
    match command.output() {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            Ok(if text.is_empty() {
                None
            } else {
                Some(PathBuf::from(text))
            })
        }
        Ok(output) if output.status.code() == Some(1) => Ok(None),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            if stderr.is_empty() {
                Err(format!("{command:?} exited with {}", output.status))
            } else {
                Err(stderr)
            }
        }
        Err(error) => Err(error.to_string()),
    }
}

fn zenity(
    name: &str,
    title: &str,
    directory: &Path,
    filters: &[FileFilter],
) -> Result<Option<PathBuf>, String> {
    let mut command = Command::new(name);
    command
        .arg("--file-selection")
        .arg("--title")
        .arg(title)
        .arg("--filename")
        .arg(directory.join(""));

    for filter in filters {
        command.arg(format!(
            "--file-filter={} | {}",
            filter.name,
            filter.globs.join(" ")
        ));
    }

    // Keep the chooser usable even for images with unusual names: zenity's
    // filter is only a convenience, and we re-validate the result ourselves.
    command.arg("--confirm-overwrite");

    run(&mut command)
}

fn kdialog(
    name: &str,
    title: &str,
    directory: &Path,
    filters: &[FileFilter],
) -> Result<Option<PathBuf>, String> {
    let mut command = Command::new(name);
    command.arg("--getopenfilename").arg(directory);

    let pattern = if filters.is_empty() {
        "All files (*)".to_owned()
    } else {
        let globs: Vec<&str> = filters
            .iter()
            .flat_map(|filter| filter.globs.iter().map(String::as_str))
            .collect();
        let name = filters
            .first()
            .map(|filter| filter.name.as_str())
            .unwrap_or("Files");
        format!("{name} ({})", globs.join(" "))
    };

    command.arg(pattern).arg("--title").arg(title);
    run(&mut command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_availability_without_panicking() {
        let _ = any_available();
        let _ = helpers();
    }

    // NOTE: deliberately no test here calls `choose_file`. It would spawn a
    // real dialog on a desktop session, which must never happen during
    // `cargo test`.
}
