use crate::models::{AppConfig, WidgetPermissions};
use crate::providers::network::NetworkProxy;
use crate::providers::system::SystemMetricsProvider;
use crate::storage::StorageManager;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::Arc;

pub struct AppState {
    pub storage: Arc<StorageManager>,
    pub config: RwLock<AppConfig>,
    pub metrics: SystemMetricsProvider,
    pub network: NetworkProxy,
    /// Maps bar window label -> raw HWND for clean AppBar unregistration on close/exit
    pub active_appbars: Mutex<HashMap<String, isize>>,
}

impl AppState {
    pub fn new(storage: StorageManager) -> Self {
        let config = storage.load_config();
        Self {
            storage: Arc::new(storage),
            config: RwLock::new(config),
            metrics: SystemMetricsProvider::new(),
            network: NetworkProxy::new(),
            active_appbars: Mutex::new(HashMap::new()),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let cfg = self.config.read();
        self.storage.save_config(&cfg)
    }

    pub fn get_instance_permissions(&self, instance_id: &str) -> Option<WidgetPermissions> {
        let cfg = self.config.read();
        if let Some(w) = cfg.widgets.iter().find(|i| i.instance_id == instance_id) {
            return Some(w.granted_permissions.clone());
        }
        if let Some(b) = cfg.bars.iter().find(|i| i.instance_id == instance_id) {
            return Some(b.granted_permissions.clone());
        }
        None
    }

    pub fn get_instance_settings(
        &self,
        instance_id: &str,
    ) -> Option<HashMap<String, serde_json::Value>> {
        let cfg = self.config.read();
        if let Some(w) = cfg.widgets.iter().find(|i| i.instance_id == instance_id) {
            return Some(w.settings.clone());
        }
        if let Some(b) = cfg.bars.iter().find(|i| i.instance_id == instance_id) {
            return Some(b.settings.clone());
        }
        None
    }
}
