//! The static browser manager compiled into the executable. Serving these
//! bytes needs no resource directory, so the shared folder may contain the
//! application package itself (for example a whole home directory).
use share_http::WebBundle;

include!(concat!(env!("OUT_DIR"), "/web_bundle.rs"));

/// `None` when the executable was built without a browser export.
pub fn bundle() -> Option<WebBundle> {
    if FILES.is_empty() {
        return None;
    }
    WebBundle::from_static(FILES).ok()
}
