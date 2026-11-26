use crate::storage::StorageManager;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const DD_SDK_JS: &str = include_str!("../../src/sdk/dd-sdk.js");

pub const WIDGET_CSP: &str = "default-src 'self' dd-widget: http://dd-widget.localhost data: blob: 'unsafe-inline'; script-src 'self' dd-widget: http://dd-widget.localhost 'unsafe-inline'; style-src 'self' dd-widget: http://dd-widget.localhost 'unsafe-inline'; img-src 'self' dd-widget: http://dd-widget.localhost data: blob:; connect-src 'none'; frame-ancestors *;";

#[derive(Debug)]
pub struct ProtocolPayload {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

/// Resolves a `dd-widget://<widget_id>/<rel_path>` request safely against the installed widgets
/// directory, serving `/__dd__/sdk.js` directly and rejecting path-traversal attempts.
pub fn resolve_widget_asset(
    storage: &StorageManager,
    widget_id: &str,
    rel_path: &str,
) -> ProtocolPayload {
    let clean_rel = rel_path.trim_start_matches('/');
    if clean_rel == "__dd__/sdk.js" {
        return ProtocolPayload {
            status: 200,
            content_type: "application/javascript; charset=utf-8".into(),
            body: DD_SDK_JS.as_bytes().to_vec(),
        };
    }

    let Some(widget_root) = storage.get_widget_dir(widget_id) else {
        return ProtocolPayload {
            status: 404,
            content_type: "text/plain; charset=utf-8".into(),
            body: format!("Widget '{}' not found", widget_id).into_bytes(),
        };
    };

    let target_rel = if clean_rel.is_empty() {
        "index.html"
    } else {
        clean_rel
    };

    let Some(safe_path) = safe_join(&widget_root, target_rel) else {
        return ProtocolPayload {
            status: 403,
            content_type: "text/plain; charset=utf-8".into(),
            body: b"Forbidden path".to_vec(),
        };
    };

    let Ok(mut bytes) = fs::read(&safe_path) else {
        return ProtocolPayload {
            status: 404,
            content_type: "text/plain; charset=utf-8".into(),
            body: b"Asset not found".to_vec(),
        };
    };

    let mime = mime_guess::from_path(&safe_path)
        .first_or_octet_stream()
        .essence_str()
        .to_string();

    // Auto-inject the dd SDK script into HTML files if the widget author didn't include it manually
    if mime == "text/html" {
        if let Ok(html) = String::from_utf8(bytes.clone()) {
            if !html.contains("__dd__/sdk.js") {
                let tag = r#"<script src="/__dd__/sdk.js"></script>"#;
                let injected = if let Some(pos) = html.find("</head>") {
                    format!("{}{}{}", &html[..pos], tag, &html[pos..])
                } else {
                    format!("{}{}", tag, html)
                };
                bytes = injected.into_bytes();
            }
        }
    }

    ProtocolPayload {
        status: 200,
        content_type: if mime.starts_with("text/") || mime == "application/javascript" {
            format!("{}; charset=utf-8", mime)
        } else {
            mime
        },
        body: bytes,
    }
}

pub fn safe_join(root: &Path, rel: &str) -> Option<PathBuf> {
    let mut out = root.to_path_buf();
    for comp in Path::new(rel).components() {
        match comp {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    let canon_root = fs::canonicalize(root).ok()?;
    let canon_out = fs::canonicalize(&out).ok()?;
    if canon_out.starts_with(&canon_root) {
        Some(canon_out)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn rejects_path_traversal_and_serves_sdk() {
        let tmp = tempdir().unwrap();
        let mgr = StorageManager::new(tmp.path().to_path_buf(), None).unwrap();
        let wdir = mgr.widgets_dir().join("demo");
        fs::create_dir_all(&wdir).unwrap();
        fs::write(wdir.join("index.html"), "<html><head></head><body>Hi</body></html>").unwrap();

        let sdk = resolve_widget_asset(&mgr, "demo", "/__dd__/sdk.js");
        assert_eq!(sdk.status, 200);
        assert!(sdk.content_type.contains("javascript"));

        let ok = resolve_widget_asset(&mgr, "demo", "/index.html");
        assert_eq!(ok.status, 200);
        let body = String::from_utf8(ok.body).unwrap();
        assert!(body.contains("/__dd__/sdk.js"));

        let traversal = resolve_widget_asset(&mgr, "demo", "/../../config.json");
        assert_eq!(traversal.status, 403);
    }
}
