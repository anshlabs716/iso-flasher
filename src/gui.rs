//! Optional GTK4 window.
//!
//! Compiled only with `--features gui`, so the command-line interface carries
//! no GUI dependency at all.
//!
//! Two rules shape this file:
//!
//! 1. The window never does privileged work. It runs as the desktop user,
//!    collects a choice, and only then hands the actual write to a root step.
//! 2. Nothing blocks the GTK main loop. Device scanning, the portal file
//!    chooser and the flash itself all run on worker threads and report back
//!    through a channel.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Button, Label, ListBox, Orientation, ProgressBar};

use crate::devices::{self, BlockDevice};
use crate::select;
use crate::{flash_with_progress, Progress};

/// Shared state between the GTK callbacks and the worker threads.
struct State {
    selected_device: RefCell<Option<BlockDevice>>,
    selected_iso: RefCell<Option<PathBuf>>,
    /// Set when the user cancels a running flash.
    cancel: Arc<AtomicBool>,
    /// Progress messages coming back from the flashing thread.
    progress_tx: Option<Sender<Progress>>,
}

/// Messages the flashing worker sends back to the UI thread.
enum FlashEvent {
    Progress(Progress),
    Done(Ok),
    Failed(String),
    Cancelled,
}

/// Entry point for `iso-flasher --gui`.
pub fn run() -> i32 {
    let application = Application::builder()
        .application_id("dev.anshlabs.iso_flasher")
        .build();

    application.connect_activate(build_window);
    application.run().into()
}

fn build_window(application: &Application) {
    let window = ApplicationWindow::builder()
        .application(application)
        .title("ISO Flasher")
        .default_width(620)
        .default_height(520)
        .build();

    let state = Rc::new(State {
        selected_device: RefCell::new(None),
        selected_iso: RefCell::new(None),
        cancel: Arc::new(AtomicBool::new(false)),
        progress_tx: None,
    });

    let root = gtk::Box::new(Orientation::Vertical, 18);
    root.set_margin_top(18);
    root.set_margin_bottom(18);
    root.set_margin_start(18);
    root.set_margin_end(18);

    // --- Intro ---
    let title = Label::new(Some("Write an ISO image to a USB drive"));
    title.add_css_class("title-2");
    root.append(&title);

    let subtitle = Label::new(Some(
        "USB drives are listed from the kernel. The ISO is chosen with your \
         desktop's own file dialog.",
    ));
    subtitle.set_wrap(true);
    subtitle.set_xalign(0.0);
    root.append(&subtitle);

    // --- Device selection ---
    let device_heading = Label::new(Some("1. USB device"));
    device_heading.set_xalign(0.0);
    device_heading.add_css_class("heading");
    root.append(&device_heading);

    let device_list = ListBox::new();
    device_list.set_selection_mode(gtk::SelectionMode::Single);
    device_list.set_vexpand(true);
    root.append(&device_list);

    let refresh = Button::with_label("Refresh devices");
    let refresh_state = state.clone();
    let refresh_list = device_list.clone();
    refresh.connect_clicked(move |_| {
        populate_devices(&refresh_list, &refresh_state);
    });
    root.append(&refresh);

    populate_devices(&device_list, &state);

    // --- ISO selection ---
    let iso_heading = Label::new(Some("2. ISO image"));
    iso_heading.set_xalign(0.0);
    iso_heading.add_css_class("heading");
    root.append(&iso_heading);

    let iso_label = Label::new(Some("No image selected"));
    iso_label.set_xalign(0.0);
    iso_label.set_wrap(true);
    root.append(&iso_label);

    let choose = Button::with_label("Choose ISO…");
    let choose_state = state.clone();
    let choose_label = iso_label.clone();
    choose.connect_clicked(move |button| {
        // The portal call blocks, so it must not run on the GTK main loop.
        let state = choose_state.clone();
        let label = choose_label.clone();
        let button = button.clone();

        button.set_sensitive(false);
        label.set_text("Opening your desktop file dialog…");

        thread::spawn(move || {
            let outcome = select::choose_iso();
            glib::idle_add_local_once(move || {
                button.set_sensitive(true);

                match outcome {
                    Ok(path) => {
                        label.set_text(&path.display().to_string());
                        *state.selected_iso.borrow_mut() = Some(path);
                    }
                    Err(error) if error.is_cancelled() => {
                        label.set_text("No image selected");
                    }
                    Err(error) => {
                        label.set_text(&format!("Could not open the file dialog: {error}"));
                    }
                }
            });
        });
    });
    root.append(&choose);

    // --- Progress ---
    let progress = ProgressBar::new();
    progress.set_show_text(true);
    progress.set_text(Some("Idle"));
    root.append(&progress);

    let status = Label::new(Some(""));
    status.set_xalign(0.0);
    status.set_wrap(true);
    root.append(&status);

    // --- Actions ---
    let actions = gtk::Box::new(Orientation::Horizontal, 12);
    let flash_button = Button::with_label("Flash to USB");
    flash_button.add_css_class("destructive-action");
    let cancel_button = Button::with_label("Cancel");
    cancel_button.set_sensitive(false);

    actions.append(&flash_button);
    actions.append(&cancel_button);
    root.append(&actions);

    let flash_state = state.clone();
    let flash_progress = progress.clone();
    let flash_status = status.clone();
    let flash_cancel = cancel_button.clone();
    let flash_iso = iso_label.clone();

    flash_button.connect_clicked(move |button| {
        let device = flash_state.selected_device.borrow().clone();
        let iso = flash_state.selected_iso.borrow().clone();

        let (Some(device), Some(iso)) = (device, iso) else {
            flash_status.set_text("Choose a USB device and an ISO image first.");
            return;
        };

        // Re-validate in the UI thread so a stale selection cannot flash.
        if let Err(error) = devices::validate_target(&device.path) {
            flash_status.set_text(&format!("That device is no longer available: {error}"));
            return;
        }

        if let Err(error) = select::validate_iso(&iso) {
            flash_status.set_text(&format!("That image is not usable: {error}"));
            return;
        }

        flash_status.set_text("Authorising… ask for your password to write to a raw device.");
        button.set_sensitive(false);
        flash_cancel.set_sensitive(true);
        flash_progress.set_fraction(0.0);
        flash_progress.set_text(Some("0.0%"));

        start_flash(
            iso,
            device,
            flash_state.clone(),
            flash_progress.clone(),
            flash_status.clone(),
            flash_cancel.clone(),
        );
    });

    let cancel_state = state.clone();
    cancel_button.connect_clicked(move |_| {
        cancel_state.cancel.store(true, Ordering::SeqCst);
    });

    window.set_child(Some(&root));
    window.present();
}

/// Fill the device list with removable whole block devices.
fn populate_devices(list: &ListBox, state: &Rc<State>) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }

    let devices = devices::removable_devices();

    if devices.is_empty() {
        let empty = Label::new(Some(
            "No removable devices detected. Plug in a USB drive and press Refresh.",
        ));
        empty.set_wrap(true);
        empty.set_margin_top(12);
        empty.set_margin_bottom(12);
        list.append(&empty);
        return;
    }

    for device in devices {
        let row = gtk::Box::new(Orientation::Vertical, 2);
        row.set_margin_top(8);
        row.set_margin_bottom(8);

        let name = Label::new(Some(&format!("/dev/{}", device.name)));
        name.set_xalign(0.0);
        name.add_css_class("heading");
        row.append(&name);

        let details = Label::new(Some(&device.describe()));
        details.set_xalign(0.0);
        details.add_css_class("dim-label");
        row.append(&details);

        let picked = device.clone();
        let selected = state.clone();
        row.connect_clicked(move |_| {
            *selected.selected_device.borrow_mut() = Some(picked);
        });

        list.append(&row);
    }

    // Preselect when there is only one obvious choice.
    if list.first_child().is_some() {
        list.select_row(
            list.first_child()
                .and_then(|child| child.downcast::<ListBoxRow>().ok())
                .as_ref(),
        );
    }
}

type ListBoxRow = gtk::ListBoxRow;

/// Run the privileged flash on a worker thread, streaming progress to the UI.
#[allow(clippy::too_many_arguments)]
fn start_flash(
    iso: PathBuf,
    device: BlockDevice,
    state: Rc<State>,
    progress_bar: ProgressBar,
    status: Label,
    cancel_button: Button,
) {
    let (tx, rx): (Sender<FlashEvent>, Receiver<FlashEvent>) = channel();
    *state.progress_tx.borrow_mut() = Some(tx.clone());

    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    thread::spawn(move || {
        let total = match std::fs::metadata(&iso) {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                let _ = tx.send(FlashEvent::Failed(format!(
                    "Cannot read the image: {error}"
                )));
                return;
            }
        };

        // Unmount before writing; the user may have the drive open.
        if let Err(error) = select::unmount(&device.path) {
            let _ = tx.send(FlashEvent::Failed(format!(
                "Could not unmount the device: {error}"
            )));
            return;
        }

        let progress_tx = tx.clone();
        let result = flash_with_progress(
            &iso,
            &device.path,
            total,
            move |progress| {
                // A full channel would block the writer, so drop updates.
                let _ = progress_tx.send(FlashEvent::Progress(progress));
            },
            || cancel.load(Ordering::SeqCst),
        );

        let event = match result {
            Ok(()) => FlashEvent::Done,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => FlashEvent::Cancelled,
            Err(error) => FlashEvent::Failed(error.to_string()),
        };

        let _ = tx.send(event);
    });

    // Drain the channel on the GTK main loop, re-arming until finished.
    fn poll(
        rx: &Receiver<FlashEvent>,
        progress_bar: &ProgressBar,
        status: &Label,
        cancel_button: &Button,
        iso: &Path,
        device: &BlockDevice,
    ) -> glib::Propagation {
        let mut finished = false;

        while let Ok(event) = rx.try_recv() {
            match event {
                FlashEvent::Progress(progress) => {
                    let fraction = (progress.percentage() / 100.0).clamp(0.0, 1.0);
                    progress_bar.set_fraction(fraction);
                    progress_bar.set_text(Some(&format!("{:.1}%", progress.percentage())));
                    status.set_text(&format!(
                        "{:.1} MiB/s · about {} left",
                        progress.speed,
                        humanise_seconds(progress.eta)
                    ));
                }
                FlashEvent::Done => {
                    finished = true;
                    progress_bar.set_fraction(1.0);
                    progress_bar.set_text(Some("Done"));
                    status.set_text(&format!("Wrote {} to /dev/{}", iso.display(), device.name));
                    cancel_button.set_sensitive(false);
                }
                FlashEvent::Cancelled => {
                    finished = true;
                    status.set_text("Cancelled. The device may be partially written.");
                    cancel_button.set_sensitive(false);
                }
                FlashEvent::Failed(message) => {
                    finished = true;
                    status.set_text(&format!("Failed: {message}"));
                    cancel_button.set_sensitive(false);
                }
            }
        }

        if !finished {
            glib::timeout_add_local_once(Duration::from_millis(60), move || {
                poll(&rx, &progress_bar, &status, &cancel_button, &iso, &device)
            });
        }

        glib::Propagation::Proceed
    }

    glib::idle_add_local_once(move || {
        poll(&rx, &progress_bar, &status, &cancel_button, &iso, &device);
    });
}

/// Render a duration the way a person would say it.
fn humanise_seconds(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "a moment".to_owned();
    }

    let total = seconds as u64;
    if total < 60 {
        format!("{total}s")
    } else if total < 3600 {
        format!("{}m {:02}s", total / 60, total % 60)
    } else {
        format!("{}h {:02}m", total / 3600, (total % 3600) / 60)
    }
}

/// Whether this host could plausibly show a GTK window.
pub fn looks_graphical() -> bool {
    std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
}
