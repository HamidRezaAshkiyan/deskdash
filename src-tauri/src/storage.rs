use crate::models::{AppConfig, InstalledWidget, WidgetManifest};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct StorageManager {
    data_dir: PathBuf,
    bundled_widgets_dir: Option<PathBuf>,
}

impl StorageManager {
    pub fn new(data_dir: PathBuf, bundled_widgets_dir: Option<PathBuf>) -> Result<Self, String> {
        fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data dir {:?}: {}", data_dir, e))?;
        fs::create_dir_all(data_dir.join("widgets"))
            .map_err(|e| format!("Failed to create widgets dir: {}", e))?;
        fs::create_dir_all(data_dir.join("storage"))
            .map_err(|e| format!("Failed to create storage dir: {}", e))?;

        let mgr = Self {
            data_dir,
            bundled_widgets_dir,
        };
        mgr.seed_bundled_widgets()?;
        Ok(mgr)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn widgets_dir(&self) -> PathBuf {
        self.data_dir.join("widgets")
    }

    pub fn config_path(&self) -> PathBuf {
        self.data_dir.join("config.json")
    }

    pub fn taskbar_lock_path(&self) -> PathBuf {
        self.data_dir.join(".taskbar-hidden.lock")
    }

    pub fn widget_storage_path(&self, instance_id: &str) -> Result<PathBuf, String> {
        if !instance_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("Invalid instance_id".into());
        }
        Ok(self.data_dir.join("storage").join(format!("{}.json", instance_id)))
    }

    pub fn load_config(&self) -> AppConfig {
        let path = self.config_path();
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => AppConfig::default(),
        }
    }

    pub fn save_config(&self, config: &AppConfig) -> Result<(), String> {
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(self.config_path(), json)
            .map_err(|e| format!("Failed to write config.json: {}", e))
    }

    pub fn read_widget_kv(&self, instance_id: &str) -> Result<HashMap<String, serde_json::Value>, String> {
        let path = self.widget_storage_path(instance_id)?;
        if !path.exists() {
            return Ok(HashMap::new());
        }
        let raw = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read widget storage: {}", e))?;
        serde_json::from_str(&raw).map_err(|e| format!("Corrupted widget storage: {}", e))
    }

    pub fn write_widget_kv(
        &self,
        instance_id: &str,
        map: &HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let path = self.widget_storage_path(instance_id)?;
        let json = serde_json::to_string_pretty(map)
            .map_err(|e| format!("Failed to serialize widget storage: {}", e))?;
        fs::write(path, json).map_err(|e| format!("Failed to write widget storage: {}", e))
    }

    pub fn list_installed_widgets(&self) -> Vec<InstalledWidget> {
        let mut results = Vec::new();
        let dir = self.widgets_dir();
        let Ok(entries) = fs::read_dir(&dir) else {
            return results;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.json");
            if let Ok(raw) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<WidgetManifest>(&raw) {
                    if manifest.validate().is_ok() {
                        let builtin = self
                            .bundled_widgets_dir
                            .as_ref()
                            .map(|b| b.join(&manifest.id).exists())
                            .unwrap_or(false);
                        results.push(InstalledWidget {
                            manifest,
                            path: path.to_string_lossy().to_string(),
                            builtin,
                        });
                    }
                }
            }
        }
        results.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));
        results
    }

    pub fn get_widget_dir(&self, widget_id: &str) -> Option<PathBuf> {
        if !widget_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return None;
        }
        let path = self.widgets_dir().join(widget_id);
        if path.is_dir() {
            Some(path)
        } else {
            None
        }
    }

    pub fn get_widget_manifest(&self, widget_id: &str) -> Result<WidgetManifest, String> {
        let dir = self
            .get_widget_dir(widget_id)
            .ok_or_else(|| format!("Widget '{}' is not installed", widget_id))?;
        let raw = fs::read_to_string(dir.join("manifest.json"))
            .map_err(|e| format!("Failed to read manifest for '{}': {}", widget_id, e))?;
        let manifest: WidgetManifest = serde_json::from_str(&raw)
            .map_err(|e| format!("Invalid manifest for '{}': {}", widget_id, e))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn remove_widget(&self, widget_id: &str) -> Result<(), String> {
        let dir = self
            .get_widget_dir(widget_id)
            .ok_or_else(|| format!("Widget '{}' not found", widget_id))?;
        fs::remove_dir_all(dir).map_err(|e| format!("Failed to remove widget: {}", e))
    }

    fn seed_bundled_widgets(&self) -> Result<(), String> {
        let Some(ref bundled) = self.bundled_widgets_dir else {
            return Ok(());
        };
        if !bundled.is_dir() {
            return Ok(());
        }
        let Ok(entries) = fs::read_dir(bundled) else {
            return Ok(());
        };
        for entry in entries.flatten() {
            let src = entry.path();
            if !src.is_dir() || !src.join("manifest.json").exists() {
                continue;
            }
            let Some(name) = src.file_name() else {
                continue;
            };
            let dst = self.widgets_dir().join(name);
            if !dst.exists() {
                copy_dir_recursive(&src, &dst)?;
            }
        }
        Ok(())
    }
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Failed to create dir {:?}: {}", dst, e))?;
    let entries = fs::read_dir(src).map_err(|e| format!("Failed to read dir {:?}: {}", src, e))?;
    for entry in entries.flatten() {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {:?} -> {:?}: {}", src_path, dst_path, e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{WidgetInstance, WidgetPermissions};
    use tempfile::tempdir;

    #[test]
    fn config_round_trip_and_kv_storage() {
        let tmp = tempdir().unwrap();
        let mgr = StorageManager::new(tmp.path().to_path_buf(), None).unwrap();

        let mut cfg = AppConfig::default();
        cfg.settings.grid_snap = 16;
        cfg.widgets.push(WidgetInstance {
            instance_id: "inst-1".into(),
            widget_id: "clock".into(),
            monitor_id: "primary".into(),
            x: 100.0,
            y: 200.0,
            width: 300.0,
            height: 150.0,
            z_index: 1,
            opacity: 0.95,
            locked: false,
            enabled: true,
            settings: HashMap::new(),
            granted_permissions: WidgetPermissions::default(),
        });

        mgr.save_config(&cfg).unwrap();
        let loaded = mgr.load_config();
        assert_eq!(loaded.settings.grid_snap, 16);
        assert_eq!(loaded.widgets.len(), 1);
        assert_eq!(loaded.widgets[0].widget_id, "clock");

        let mut kv = HashMap::new();
        kv.insert("note".into(), serde_json::json!("Buy milk"));
        mgr.write_widget_kv("inst-1", &kv).unwrap();
        let read_back = mgr.read_widget_kv("inst-1").unwrap();
        assert_eq!(read_back.get("note"), Some(&serde_json::json!("Buy milk")));
    }

    #[test]
    fn bundled_starter_widgets_seed_and_validate() {
        let tmp = tempdir().unwrap();
        let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("resources")
            .join("widgets");
        let mgr = StorageManager::new(tmp.path().to_path_buf(), Some(bundled)).unwrap();
        let list = mgr.list_installed_widgets();
        assert_eq!(list.len(), 7, "Expected all 7 starter widgets/bars to be valid");
        let ids: Vec<&str> = list.iter().map(|w| w.manifest.id.as_str()).collect();
        for expected in [
            "clock",
            "weather",
            "system-monitor",
            "media-player",
            "sticky-notes",
            "floating-dock",
            "top-status-bar",
        ] {
            assert!(ids.contains(&expected), "Missing starter widget '{}'", expected);
        }
    }
}
