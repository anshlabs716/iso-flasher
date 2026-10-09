//! XDG Desktop Portal file chooser (`org.freedesktop.portal.FileChooser`).
//!
//! This is the standards-based way to ask the desktop for a file. Whichever
//! portal backend the user has configured supplies the actual dialog, so the
//! same code works on KDE Plasma, GNOME, Xfce, Cinnamon, MATE, Budgie, LXQt,
//! COSMIC and anything else that ships a FileChooser portal, on Wayland and X11
//! alike. We never name a specific file manager here.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use zbus::blocking::{connection::Builder, Connection};
use zbus::zvariant::{Array, OwnedObjectPath, Signature, Value};

use crate::session::BusAddress;

const PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";

/// Response codes defined by `org.freedesktop.portal.Request`.
const RESPONSE_SUCCESS: u32 = 0;
const RESPONSE_CANCELLED: u32 = 1;

#[zbus::proxy(
    interface = "org.freedesktop.portal.FileChooser",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop",
    gen_blocking = true
)]
trait FileChooser {
    fn open_file(
        &self,
        parent_window: &str,
        title: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.freedesktop.portal.Request",
    default_service = "org.freedesktop.portal.Desktop",
    gen_blocking = true
)]
trait Request {
    fn close(&self) -> zbus::Result<()>;

    #[zbus(signal)]
    fn response(
        &self,
        response: u32,
        results: HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::Result<()>;
}

/// Why a file could not be chosen.
#[derive(Debug)]
pub enum PickerError {
    /// The user dismissed the dialog. This is a normal outcome, not a failure.
    Cancelled,
    /// No portal, or the portal refused to start a dialog.
    Unavailable(String),
    /// The portal started but misbehaved.
    Portal(String),
}

impl std::fmt::Display for PickerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PickerError::Cancelled => write!(formatter, "cancelled"),
            PickerError::Unavailable(detail) => write!(formatter, "portal unavailable: {detail}"),
            PickerError::Portal(detail) => write!(formatter, "{detail}"),
        }
    }
}

impl std::error::Error for PickerError {}

/// A glob or MIME filter offered in the chooser.
#[derive(Clone, Debug)]
pub struct FileFilter {
    pub name: String,
    /// Glob patterns such as `*.iso`.
    pub globs: Vec<String>,
    /// MIME types such as `application/x-cd-image`.
    pub mime_types: Vec<String>,
}

fn signature(text: &str) -> Signature {
    Signature::try_from(text).expect("static D-Bus signature must be valid")
}

/// Unique per call, as required by the portal's `handle_token` option.
fn handle_token() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_millis())
        .unwrap_or_default();
    format!("isoflasher{millis}_{}", std::process::id())
}

/// Build the portal's `filters` option, shaped `a(sa(us))`.
///
/// Each entry is `(name, [(kind, pattern)])` where kind 0 is a case-sensitive
/// glob and kind 1 is a MIME type.
fn build_filters(filters: &[FileFilter]) -> Value<'static> {
    let mut entries = Array::new(&signature("(sa(us))"));

    for filter in filters {
        let mut patterns = Array::new(&signature("(us)"));

        for glob in &filter.globs {
            let owned = zbus::zvariant::Str::from(glob.clone());
            let pair = zbus::zvariant::Structure::from((0_u32, owned));
            let _ = patterns.append(zbus::zvariant::Value::new(pair));
        }

        for mime in &filter.mime_types {
            let owned = zbus::zvariant::Str::from(mime.clone());
            let pair = zbus::zvariant::Structure::from((1_u32, owned));
            let _ = patterns.append(zbus::zvariant::Value::new(pair));
        }

        let name = zbus::zvariant::Str::from(filter.name.clone());
        let entry = zbus::zvariant::Structure::from((name, patterns));
        let _ = entries.append(zbus::zvariant::Value::new(entry));
    }

    Value::Array(entries)
}

/// Convert a `file://` URI from the portal into a filesystem path.
///
/// Deliberately handles only the cases we can actually produce and validate:
/// `file:///path/to/file`. Other schemes are rejected rather than guessed at.
pub fn uri_to_path(uri: &str) -> Option<std::path::PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    // `file://host/path` is legal but we have no business accepting a remote host.
    let path = match rest.find('/') {
        Some(0) => rest,
        Some(_) | None => return None,
    };

    let decoded = percent_decode(path)?;
    if decoded.is_empty() {
        return None;
    }

    Some(std::path::PathBuf::from(decoded))
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = input.get(index + 1..index + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(out).ok()
}

/// Ask the desktop to show its file chooser and return what the user picked.
///
/// `directory` is used only as a suggestion for where the dialog opens; it is
/// not a constraint, so the user may navigate anywhere.
pub fn choose_file(
    title: &str,
    directory: &std::path::Path,
    filters: &[FileFilter],
) -> Result<std::path::PathBuf, PickerError> {
    let connection = connect().map_err(PickerError::Unavailable)?;

    let token = handle_token();
    let mut options: HashMap<&str, Value<'_>> = HashMap::new();
    options.insert("handle_token", Value::from(token.as_str()));
    options.insert("multiple", Value::from(false));

    if !filters.is_empty() {
        options.insert("filters", build_filters(filters));
        // Ask for the first filter to be preselected.
        if let Some(first) = filters.first() {
            options.insert("current_filter", build_current_filter(first));
        }
    }

    if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
        if parent.is_dir() {
            if let Some(parent) = parent.to_str() {
                options.insert("directory", Value::from(parent));
            }
        }
    }

    let chooser = FileChooserProxyBlocking::new(&connection)
        .map_err(|error| PickerError::Unavailable(error.to_string()))?;

    let handle = chooser
        .open_file("", title, options)
        .map_err(|error| PickerError::Unavailable(error.to_string()))?;

    let request = RequestProxyBlocking::builder(&connection)
        .destination(PORTAL_BUS_NAME)
        .map_err(|error| PickerError::Portal(error.to_string()))?
        .path(handle.clone())
        .map_err(|error| PickerError::Portal(error.to_string()))?
        .build()
        .map_err(|error| PickerError::Portal(error.to_string()))?;

    // Subscribe before waiting so the reply cannot be missed.
    let mut responses = request
        .receive_response()
        .map_err(|error| PickerError::Portal(error.to_string()))?;

    let Some(message) = responses.next() else {
        let _ = request.close();
        return Err(PickerError::Portal(
            "portal closed without responding".to_owned(),
        ));
    };

    let args = message
        .args()
        .map_err(|error| PickerError::Portal(error.to_string()))?;
    let (code, results) = (args.response, args.results);

    parse_response(code, results)
}

fn build_current_filter(filter: &FileFilter) -> Value<'static> {
    let mut patterns = Array::new(&signature("(us)"));
    for glob in &filter.globs {
        let _ = patterns.append(zbus::zvariant::Value::new(zbus::zvariant::Structure::from(
            (0_u32, zbus::zvariant::Str::from(glob.clone())),
        )));
    }
    for mime in &filter.mime_types {
        let _ = patterns.append(zbus::zvariant::Value::new(zbus::zvariant::Structure::from(
            (1_u32, zbus::zvariant::Str::from(mime.clone())),
        )));
    }

    let name = zbus::zvariant::Str::from(filter.name.clone());
    let entry = zbus::zvariant::Structure::from((name, patterns));
    zbus::zvariant::Value::new(entry)
}

/// Interpret a decoded `Response` signal: a response code plus a results map.
///
/// Typed decoding matters here: the raw signal body is `(ua{sv})`, so decoding
/// it as a bare `Value` fails with a signature mismatch and every successful
/// selection would be reported as an error.
pub fn parse_response(
    code: u32,
    results: HashMap<String, zbus::zvariant::OwnedValue>,
) -> Result<std::path::PathBuf, PickerError> {
    match code {
        RESPONSE_SUCCESS => {}
        RESPONSE_CANCELLED => return Err(PickerError::Cancelled),
        // RESPONSE_OTHER and anything a future backend invents.
        _ => return Err(PickerError::Portal("portal refused the request".to_owned())),
    }

    if let Some(uris) = results.get("uris") {
        // `uris` is an array of strings wrapped in a variant.
        let owned = uris.clone();
        if let Ok(list) = <Vec<String> as TryFrom<zbus::zvariant::OwnedValue>>::try_from(owned) {
            for uri in list {
                if let Some(path) = uri_to_path(&uri) {
                    return Ok(path);
                }
            }
        }
    }

    // Some backends return a successful-but-empty response when the user
    // dismisses without choosing; treat that as a cancellation, not a crash.
    Err(PickerError::Cancelled)
}

fn connect() -> Result<Connection, String> {
    let builder = match crate::session::session_bus_address() {
        BusAddress::Explicit(address) => {
            Builder::address(address.as_str()).map_err(|e| e.to_string())?
        }
        BusAddress::Unavailable => return Err("no session bus address available".to_owned()),
    };

    builder.build().map_err(|error| error.to_string())
}

/// True when a FileChooser portal appears to be usable right now.
pub fn is_available() -> bool {
    connect().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn decodes_plain_file_uri() {
        assert_eq!(
            uri_to_path("file:///home/ansh/TinyCore.iso"),
            Some(PathBuf::from("/home/ansh/TinyCore.iso"))
        );
    }

    #[test]
    fn decodes_percent_escapes() {
        assert_eq!(
            uri_to_path("file:///tmp/My%20Disk.iso"),
            Some(PathBuf::from("/tmp/My Disk.iso"))
        );
    }

    #[test]
    fn rejects_non_file_and_remote_uris() {
        assert_eq!(uri_to_path("https://example.com/a.iso"), None);
        assert_eq!(uri_to_path("file://host/share/a.iso"), None);
        assert_eq!(uri_to_path("file://"), None);
        assert_eq!(uri_to_path("file:///tmp/%zz.iso"), None);
    }

    #[test]
    fn filters_use_the_documented_signature() {
        let filters = vec![FileFilter {
            name: "ISO images".to_owned(),
            globs: vec!["*.iso".to_owned()],
            mime_types: vec!["application/x-cd-image".to_owned()],
        }];

        let zbus::zvariant::Value::Array(entries) = build_filters(&filters) else {
            panic!("filters must be an array");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries.signature().to_string(), "a(sa(us))");
    }

    #[test]
    fn cancellation_is_not_an_error_state() {
        // Response code 1 must map to Cancelled so callers can exit quietly.
        assert!(matches!(
            parse_response(1, HashMap::new()),
            Err(PickerError::Cancelled)
        ));
    }

    #[test]
    fn successful_selection_decodes_to_a_path() {
        // This is the regression test for "press Open and nothing happens":
        // the `uris` entry arrives as a variant-wrapped array of strings, and
        // decoding it as a bare Value used to fail with a signature mismatch.
        let uris: Vec<String> = vec!["file:///tmp/TinyCore-current.iso".to_owned()];
        let mut results: HashMap<String, zbus::zvariant::OwnedValue> = HashMap::new();
        results.insert(
            "uris".to_owned(),
            zbus::zvariant::OwnedValue::try_from(Value::from(uris)).expect("own uris"),
        );

        let path = parse_response(0, results).expect("a chosen file must decode");
        assert_eq!(path, std::path::PathBuf::from("/tmp/TinyCore-current.iso"));
    }

    #[test]
    fn spaces_in_chosen_names_are_decoded() {
        let uris: Vec<String> = vec!["file:///home/ansh/My%20Linux.iso".to_owned()];
        let mut results: HashMap<String, zbus::zvariant::OwnedValue> = HashMap::new();
        results.insert(
            "uris".to_owned(),
            zbus::zvariant::OwnedValue::try_from(Value::from(uris)).expect("own uris"),
        );

        assert_eq!(
            parse_response(0, results).expect("decode"),
            std::path::PathBuf::from("/home/ansh/My Linux.iso")
        );
    }

    #[test]
    fn success_without_uris_is_treated_as_cancelled() {
        assert!(matches!(
            parse_response(0, HashMap::new()),
            Err(PickerError::Cancelled)
        ));
    }

    #[test]
    fn unknown_response_codes_are_errors_not_cancellations() {
        let outcome = parse_response(2, HashMap::new());
        assert!(matches!(outcome, Err(PickerError::Portal(_))));
    }
}
