use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceKind {
    #[default]
    Widget,
    Bar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BarEdge {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Size2D {
    pub width: f64,
    pub height: f64,
}

impl Default for Size2D {
    fn default() -> Self {
        Self {
            width: 320.0,
            height: 200.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    #[allow(dead_code)]
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetPermissions {
    #[serde(default)]
    pub system: bool,
    #[serde(default)]
    pub media: bool,
    #[serde(default)]
    pub clipboard: bool,
    #[serde(default)]
    pub taskbar: bool,
    #[serde(default)]
    pub launcher: bool,
    #[serde(default)]
    pub storage: bool,
    #[serde(default)]
    pub notifications: bool,
    #[serde(default)]
    pub network: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingOption {
    pub label: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String, // "string" | "number" | "boolean" | "select" | "color"
    #[serde(default)]
    pub default: serde_json::Value,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub options: Option<Vec<SettingOption>>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarConfig {
    #[serde(default)]
    pub edge: BarEdge,
    #[serde(default = "default_bar_thickness")]
    pub thickness: f64,
    #[serde(default = "default_true")]
    pub reserve_space: bool,
    #[serde(default)]
    pub hide_native_taskbar: bool,
}

fn default_bar_thickness() -> f64 {
    56.0
}

fn default_true() -> bool {
    true
}

fn default_entry() -> String {
    "index.html".to_string()
}

fn default_opacity() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub kind: SurfaceKind,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub default_size: Size2D,
    #[serde(default)]
    pub min_size: Option<Size2D>,
    #[serde(default)]
    pub max_size: Option<Size2D>,
    #[serde(default)]
    pub permissions: WidgetPermissions,
    #[serde(default)]
    pub settings_schema: Vec<SettingField>,
    #[serde(default)]
    pub bar: Option<BarConfig>,
}

impl WidgetManifest {
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("Widget manifest id cannot be empty".into());
        }
        if !self
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(format!(
                "Invalid widget id '{}': only ASCII alphanumeric, '-', and '_' are allowed",
                self.id
            ));
        }
        if self.name.trim().is_empty() {
            return Err("Widget manifest name cannot be empty".into());
        }
        if self.entry.contains("..") || self.entry.starts_with('/') || self.entry.starts_with('\\')
        {
            return Err(format!("Unsafe widget entry path '{}'", self.entry));
        }
        Ok(())
    }

    pub fn default_settings_map(&self) -> HashMap<String, serde_json::Value> {
        self.settings_schema
            .iter()
            .map(|f| (f.key.clone(), f.default.clone()))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledWidget {
    pub manifest: WidgetManifest,
    pub path: String,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetInstance {
    pub instance_id: String,
    pub widget_id: String,
    pub monitor_id: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub z_index: i32,
    #[serde(default = "default_opacity")]
    pub opacity: f64,
    #[serde(default)]
    pub locked: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub granted_permissions: WidgetPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BarInstance {
    pub instance_id: String,
    pub widget_id: String,
    pub monitor_id: String,
    #[serde(default)]
    pub edge: BarEdge,
    #[serde(default = "default_bar_thickness")]
    pub thickness: f64,
    #[serde(default = "default_true")]
    pub reserve_space: bool,
    #[serde(default)]
    pub hide_native_taskbar: bool,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub granted_permissions: WidgetPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_grid_snap")]
    pub grid_snap: u32,
    #[serde(default = "default_edit_hotkey")]
    pub edit_mode_hotkey: String,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default = "default_true")]
    pub auto_pause_fullscreen: bool,
    #[serde(default = "default_true")]
    pub minimize_to_tray: bool,
    #[serde(default = "default_registries")]
    pub registry_urls: Vec<String>,
}

fn default_grid_snap() -> u32 {
    12
}

fn default_edit_hotkey() -> String {
    "Ctrl+Alt+E".to_string()
}

fn default_registries() -> Vec<String> {
    vec!["https://raw.githubusercontent.com/HamidRezaAshkiyan/deskdash-registry/main/registry.json".to_string()]
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            grid_snap: default_grid_snap(),
            edit_mode_hotkey: default_edit_hotkey(),
            autostart: false,
            auto_pause_fullscreen: true,
            minimize_to_tray: true,
            registry_urls: default_registries(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default = "default_profile_name")]
    pub active_profile: String,
    #[serde(default)]
    pub edit_mode: bool,
    #[serde(default)]
    pub settings: AppSettings,
    #[serde(default)]
    pub widgets: Vec<WidgetInstance>,
    #[serde(default)]
    pub bars: Vec<BarInstance>,
}

fn default_profile_name() -> String {
    "Default".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            active_profile: default_profile_name(),
            edit_mode: false,
            settings: AppSettings::default(),
            widgets: Vec::new(),
            bars: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub id: String,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskStat {
    pub name: String,
    pub mount_point: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub cpu_usage: f32,
    pub cpu_cores: Vec<f32>,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_percent: f32,
    pub gpu_usage: Option<f32>,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub disks: Vec<DiskStat>,
    pub uptime_secs: u64,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSession {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub source_app: String,
    pub thumbnail_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub id: String,
    pub title: String,
    pub app_name: String,
    pub exe_path: String,
    pub is_focused: bool,
    pub is_minimized: bool,
    pub icon_data_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTheme {
    pub is_dark: bool,
    pub accent_hex: String,
}

impl Default for SystemTheme {
    fn default() -> Self {
        Self {
            is_dark: true,
            accent_hex: "#6366f1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketplaceItem {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub category: String,
    pub kind: SurfaceKind,
    pub download_url: Option<String>,
    pub sha256: Option<String>,
    pub permissions: WidgetPermissions,
    pub builtin: bool,
    pub installed: bool,
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionAudit {
    pub widget_id: String,
    pub widget_name: String,
    pub requested: WidgetPermissions,
    pub added_capabilities: Vec<String>,
    pub added_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DdInvokeRequest {
    pub instance_id: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DdInvokeResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl DdInvokeResponse {
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}
