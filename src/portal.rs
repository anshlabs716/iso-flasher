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
    #[zbus(signal)]
    fn response(&self, response: u32, results: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
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

    let request = zbus::blocking::Proxy::new(
        &connection,
        PORTAL_BUS_NAME,
        handle.as_ref(),
        "org.freedesktop.portal.Request",
    )
    .map_err(|error| PickerError::Portal(error.to_string()))?;

    // Subscribe before returning so the reply cannot be missed.
    let mut responses = request
        .receive_signal("Response")
        .map_err(|error| PickerError::Portal(error.to_string()))?;

    let Some(message) = responses.next() else {
        let _ = request.call_method("Close", &());
        return Err(PickerError::Portal(
            "portal closed without responding".to_owned(),
        ));
    };

    let _ = request.call_method("Close", &());

    let body = message.body();
    let payload: Value<'_> = body
        .deserialize()
        .map_err(|error| PickerError::Portal(error.to_string()))?;

    parse_response(&payload)
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

/// Interpret the `Response` signal payload: `(u response, a{sv} results)`.
fn parse_response(payload: &zbus::zvariant::Value<'_>) -> Result<std::path::PathBuf, PickerError> {
    let zbus::zvariant::Value::Structure(fields) = payload else {
        return Err(PickerError::Portal("malformed response".to_owned()));
    };

    let fields = fields.fields();
    let Some(code) = fields
        .first()
        .and_then(|value| value.downcast_ref::<u32>().ok())
    else {
        return Err(PickerError::Portal("missing response code".to_owned()));
    };

    match code {
        RESPONSE_SUCCESS => {}
        RESPONSE_CANCELLED => return Err(PickerError::Cancelled),
        // RESPONSE_OTHER and anything a future backend invents.
        _ => {
            let detail = fields
                .get(1)
                .and_then(|value| {
                    value
                        .downcast_ref::<zbus::zvariant::Str>()
                        .ok()
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "unspecified portal error".to_owned());
            return Err(PickerError::Portal(format!(
                "portal refused the request: {detail}"
            )));
        }
    }

    let results = fields
        .get(1)
        .and_then(|value| value.downcast_ref::<zbus::zvariant::Dict<'_, '_>>().ok());
    let Some(results) = results else {
        return Err(PickerError::Portal(
            "response carried no results".to_owned(),
        ));
    };

    for (key, value) in results.iter() {
        let is_uris = matches!(
            key.downcast_ref::<zbus::zvariant::Str>()
                .ok()
                .map(|name| name.to_string())
                .as_deref(),
            Some("uris")
        );
        if !is_uris {
            continue;
        }

        let zbus::zvariant::Value::Array(uris) = value else {
            continue;
        };

        for uri in uris.iter() {
            let Some(text) = uri.downcast_ref::<zbus::zvariant::Str>().ok() else {
                continue;
            };
            if let Some(path) = uri_to_path(text.as_str()) {
                return Ok(path);
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
        // Response payload: (1, {}) -- cancelled with no results.
        let payload = zbus::zvariant::Structure::from((1_u32, HashMap::<&str, Value<'_>>::new()));
        let value = zbus::zvariant::Value::new(payload);
        assert!(matches!(
            parse_response(&value),
            Err(PickerError::Cancelled)
        ));
    }
}
