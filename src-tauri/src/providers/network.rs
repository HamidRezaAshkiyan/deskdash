use crate::models::WidgetPermissions;
use crate::permissions::PermissionGate;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024; // 2 MB cap per widget request

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyFetchRequest {
    pub url: String,
    #[serde(default = "default_get")]
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

fn default_get() -> String {
    "GET".to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyFetchResponse {
    pub status: u16,
    pub ok: bool,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct NetworkProxy {
    client: reqwest::Client,
}

impl NetworkProxy {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("DeskDash/0.1.0")
            .build()
            .expect("Failed to initialize reqwest client");
        Self { client }
    }

    pub async fn fetch(
        &self,
        granted: &WidgetPermissions,
        req: ProxyFetchRequest,
    ) -> Result<ProxyFetchResponse, String> {
        let url = PermissionGate::require_network(granted, &req.url)?;
        let method = Method::from_bytes(req.method.to_uppercase().as_bytes())
            .map_err(|_| format!("Invalid HTTP method '{}'", req.method))?;

        let mut builder = self.client.request(method, url);
        for (k, v) in &req.headers {
            builder = builder.header(k.as_str(), v.as_str());
        }
        if let Some(body) = req.body {
            builder = builder.body(body);
        }

        let resp = builder
            .send()
            .await
            .map_err(|e| format!("Network request failed: {}", e))?;

        let status = resp.status().as_u16();
        let ok = resp.status().is_success();
        let mut headers = HashMap::new();
        for (k, v) in resp.headers() {
            if let Ok(val) = v.to_str() {
                headers.insert(k.to_string(), val.to_string());
            }
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Failed reading response body: {}", e))?;
        if bytes.len() > MAX_BODY_BYTES {
            return Err(format!(
                "Response exceeded max size of {} bytes",
                MAX_BODY_BYTES
            ));
        }

        let body = String::from_utf8_lossy(&bytes).to_string();
        Ok(ProxyFetchResponse {
            status,
            ok,
            headers,
            body,
        })
    }
}
