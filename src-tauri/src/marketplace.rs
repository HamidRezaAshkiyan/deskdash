use crate::models::{InstalledWidget, MarketplaceItem, WidgetManifest};
use crate::storage::{copy_dir_recursive, StorageManager};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Component, Path};
use zip::ZipArchive;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteRegistry {
    #[serde(default)]
    items: Vec<RemoteRegistryItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteRegistryItem {
    id: String,
    name: String,
    version: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    kind: crate::models::SurfaceKind,
    download_url: Option<String>,
    sha256: Option<String>,
    #[serde(default)]
    permissions: crate::models::WidgetPermissions,
}

pub async fn fetch_marketplace_catalog(
    storage: &StorageManager,
    registry_urls: &[String],
) -> Vec<MarketplaceItem> {
    let installed = storage.list_installed_widgets();
    let installed_map: std::collections::HashMap<String, &InstalledWidget> = installed
        .iter()
        .map(|w| (w.manifest.id.clone(), w))
        .collect();

    let mut items: Vec<MarketplaceItem> = installed
        .iter()
        .map(|w| MarketplaceItem {
            id: w.manifest.id.clone(),
            name: w.manifest.name.clone(),
            version: w.manifest.version.clone(),
            author: w.manifest.author.clone(),
            description: w.manifest.description.clone(),
            category: if w.manifest.category.is_empty() {
                "General".into()
            } else {
                w.manifest.category.clone()
            },
            kind: w.manifest.kind,
            download_url: None,
            sha256: None,
            permissions: w.manifest.permissions.clone(),
            builtin: w.builtin,
            installed: true,
            installed_version: Some(w.manifest.version.clone()),
        })
        .collect();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .user_agent("DeskDash/0.1.0")
        .build()
        .ok();

    if let Some(client) = client {
        for url in registry_urls {
            if let Ok(resp) = client.get(url).send().await {
                if let Ok(reg) = resp.json::<RemoteRegistry>().await {
                    for remote in reg.items {
                        if let Some(existing) = items.iter_mut().find(|i| i.id == remote.id) {
                            existing.download_url = remote.download_url;
                            existing.sha256 = remote.sha256;
                            existing.version = remote.version;
                        } else {
                            let inst = installed_map.get(&remote.id);
                            items.push(MarketplaceItem {
                                id: remote.id,
                                name: remote.name,
                                version: remote.version,
                                author: remote.author,
                                description: remote.description,
                                category: if remote.category.is_empty() {
                                    "Community".into()
                                } else {
                                    remote.category
                                },
                                kind: remote.kind,
                                download_url: remote.download_url,
                                sha256: remote.sha256,
                                permissions: remote.permissions,
                                builtin: false,
                                installed: inst.is_some(),
                                installed_version: inst.map(|w| w.manifest.version.clone()),
                            });
                        }
                    }
                }
            }
        }
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    items
}

pub async fn install_from_url(
    storage: &StorageManager,
    download_url: &str,
    expected_sha256: Option<&str>,
) -> Result<WidgetManifest, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("DeskDash/0.1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let bytes = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download widget package: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed reading widget bytes: {}", e))?;

    if let Some(expected) = expected_sha256 {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = hex::encode(hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected.trim()) {
            return Err(format!(
                "SHA-256 mismatch: expected {}, got {}",
                expected, actual
            ));
        }
    }

    install_from_zip_bytes(storage, &bytes)
}

pub fn install_from_local_path(
    storage: &StorageManager,
    source_path: &Path,
) -> Result<WidgetManifest, String> {
    if source_path.is_file()
        && source_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| e.eq_ignore_ascii_case("zip"))
            .unwrap_or(false)
    {
        let bytes = fs::read(source_path)
            .map_err(|e| format!("Failed to read zip {:?}: {}", source_path, e))?;
        return install_from_zip_bytes(storage, &bytes);
    }

    if source_path.is_dir() {
        let manifest_path = source_path.join("manifest.json");
        let raw = fs::read_to_string(&manifest_path)
            .map_err(|_| "Selected folder does not contain a valid manifest.json".to_string())?;
        let manifest: WidgetManifest =
            serde_json::from_str(&raw).map_err(|e| format!("Invalid manifest.json: {}", e))?;
        manifest.validate()?;

        let dst = storage.widgets_dir().join(&manifest.id);
        if dst.exists() {
            fs::remove_dir_all(&dst).map_err(|e| e.to_string())?;
        }
        copy_dir_recursive(source_path, &dst)?;
        return Ok(manifest);
    }

    Err("Please select a widget folder containing manifest.json or a .zip package".into())
}

pub fn install_from_zip_bytes(
    storage: &StorageManager,
    zip_bytes: &[u8],
) -> Result<WidgetManifest, String> {
    let reader = Cursor::new(zip_bytes);
    let mut archive =
        ZipArchive::new(reader).map_err(|e| format!("Invalid zip archive: {}", e))?;

    // First pass: validate all entries for path traversal and locate manifest.json
    let mut manifest_raw: Option<String> = None;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        if !is_safe_zip_path(&name) {
            return Err(format!("Blocked unsafe path inside zip: '{}'", name));
        }
        if name == "manifest.json" {
            let mut s = String::new();
            entry.read_to_string(&mut s).map_err(|e| e.to_string())?;
            manifest_raw = Some(s);
        }
    }

    let manifest_str =
        manifest_raw.ok_or_else(|| "Widget zip package is missing root manifest.json".to_string())?;
    let manifest: WidgetManifest =
        serde_json::from_str(&manifest_str).map_err(|e| format!("Invalid manifest.json: {}", e))?;
    manifest.validate()?;

    let dst = storage.widgets_dir().join(&manifest.id);
    if dst.exists() {
        fs::remove_dir_all(&dst).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&dst).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let rel = entry.name().to_string();
        let out_path = dst.join(&rel);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
            fs::write(&out_path, buf).map_err(|e| e.to_string())?;
        }
    }

    Ok(manifest)
}

fn is_safe_zip_path(name: &str) -> bool {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    for comp in Path::new(name).components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    #[test]
    fn rejects_zip_slip_and_installs_valid_zip() {
        let tmp = tempdir().unwrap();
        let mgr = StorageManager::new(tmp.path().to_path_buf(), None).unwrap();

        // Malicious zip with ../ traversal
        let mut bad_buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut bad_buf);
            zw.start_file("../evil.txt", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(b"pwned").unwrap();
            zw.finish().unwrap();
        }
        assert!(install_from_zip_bytes(&mgr, &bad_buf.into_inner()).is_err());

        // Valid widget zip
        let mut ok_buf = Cursor::new(Vec::new());
        {
            let mut zw = zip::ZipWriter::new(&mut ok_buf);
            zw.start_file("manifest.json", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(
                br#"{"id":"hello-widget","name":"Hello","version":"1.0.0","kind":"widget"}"#,
            )
            .unwrap();
            zw.start_file("index.html", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(b"<html><body>Hello</body></html>").unwrap();
            zw.finish().unwrap();
        }
        let manifest = install_from_zip_bytes(&mgr, &ok_buf.into_inner()).unwrap();
        assert_eq!(manifest.id, "hello-widget");
        assert!(mgr.widgets_dir().join("hello-widget/index.html").exists());
    }
}
