use crate::models::{PermissionAudit, WidgetManifest, WidgetPermissions};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    System,
    Media,
    Clipboard,
    Taskbar,
    Launcher,
    Storage,
    #[allow(dead_code)]
    Notifications,
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::System => "system",
            Capability::Media => "media",
            Capability::Clipboard => "clipboard",
            Capability::Taskbar => "taskbar",
            Capability::Launcher => "launcher",
            Capability::Storage => "storage",
            Capability::Notifications => "notifications",
        }
    }
}

pub struct PermissionGate;

impl PermissionGate {
    /// Default-deny capability check against a widget/bar instance's granted permissions.
    pub fn require(granted: &WidgetPermissions, cap: Capability) -> Result<(), String> {
        let allowed = match cap {
            Capability::System => granted.system,
            Capability::Media => granted.media,
            Capability::Clipboard => granted.clipboard,
            Capability::Taskbar => granted.taskbar,
            Capability::Launcher => granted.launcher,
            Capability::Storage => granted.storage,
            Capability::Notifications => granted.notifications,
        };
        if allowed {
            Ok(())
        } else {
            Err(format!(
                "Permission denied: capability '{}' has not been granted to this widget",
                cap.as_str()
            ))
        }
    }

    /// Validates that a network request URL uses an allowed scheme (`https`, or `http` only for
    /// explicitly allowlisted `localhost`/`127.0.0.1`) and matches the granted domain allowlist.
    pub fn require_network(granted: &WidgetPermissions, raw_url: &str) -> Result<Url, String> {
        let parsed = Url::parse(raw_url).map_err(|e| format!("Invalid URL '{}': {}", raw_url, e))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| "URL must contain a valid host".to_string())?
            .to_ascii_lowercase();

        let scheme = parsed.scheme();
        let is_local = host == "localhost" || host == "127.0.0.1";
        if scheme != "https" && !(scheme == "http" && is_local) {
            return Err(format!(
                "Blocked URL scheme '{}': only https:// (or http://localhost) is permitted",
                scheme
            ));
        }

        let matched = granted.network.iter().any(|pattern| host_matches(&host, pattern));
        if matched {
            Ok(parsed)
        } else {
            Err(format!(
                "Permission denied: network host '{}' is not in this widget's allowlist",
                host
            ))
        }
    }

    /// Computes newly requested capabilities/domains when installing or updating a widget.
    pub fn audit(manifest: &WidgetManifest, previous: Option<&WidgetPermissions>) -> PermissionAudit {
        let empty = WidgetPermissions::default();
        let prev = previous.unwrap_or(&empty);
        let req = &manifest.permissions;

        let mut added_capabilities = Vec::new();
        let checks = [
            (req.system && !prev.system, "system"),
            (req.media && !prev.media, "media"),
            (req.clipboard && !prev.clipboard, "clipboard"),
            (req.taskbar && !prev.taskbar, "taskbar"),
            (req.launcher && !prev.launcher, "launcher"),
            (req.storage && !prev.storage, "storage"),
            (req.notifications && !prev.notifications, "notifications"),
        ];
        for (is_new, name) in checks {
            if is_new {
                added_capabilities.push(name.to_string());
            }
        }

        let added_domains = req
            .network
            .iter()
            .filter(|d| !prev.network.iter().any(|p| p.eq_ignore_ascii_case(d)))
            .cloned()
            .collect();

        PermissionAudit {
            widget_id: manifest.id.clone(),
            widget_name: manifest.name.clone(),
            requested: req.clone(),
            added_capabilities,
            added_domains,
        }
    }
}

fn host_matches(host: &str, pattern: &str) -> bool {
    let pat = pattern.trim().to_ascii_lowercase();
    if pat.is_empty() || pat == "*" {
        return false;
    }
    if let Some(suffix) = pat.strip_prefix("*.") {
        host != suffix && host.ends_with(&format!(".{}", suffix))
    } else {
        host == pat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_deny_blocks_ungranted_capabilities() {
        let perms = WidgetPermissions::default();
        assert!(PermissionGate::require(&perms, Capability::System).is_err());
        assert!(PermissionGate::require(&perms, Capability::Taskbar).is_err());
        assert!(PermissionGate::require(&perms, Capability::Media).is_err());
    }

    #[test]
    fn grants_matching_capabilities() {
        let perms = WidgetPermissions {
            system: true,
            storage: true,
            ..Default::default()
        };
        assert!(PermissionGate::require(&perms, Capability::System).is_ok());
        assert!(PermissionGate::require(&perms, Capability::Storage).is_ok());
        assert!(PermissionGate::require(&perms, Capability::Clipboard).is_err());
    }

    #[test]
    fn network_allowlist_enforces_exact_and_subdomain_and_blocks_wildcards_and_http() {
        let perms = WidgetPermissions {
            network: vec![
                "api.open-meteo.com".into(),
                "*.example.org".into(),
                "*".into(),
                "localhost".into(),
            ],
            ..Default::default()
        };

        assert!(PermissionGate::require_network(&perms, "https://api.open-meteo.com/v1/forecast").is_ok());
        assert!(PermissionGate::require_network(&perms, "https://sub.example.org/data").is_ok());
        assert!(PermissionGate::require_network(&perms, "http://localhost:8080/health").is_ok());

        // Must reject bare wildcard match, non-https remote, and unlisted hosts
        assert!(PermissionGate::require_network(&perms, "https://evil.com/exfil").is_err());
        assert!(PermissionGate::require_network(&perms, "http://api.open-meteo.com/v1/forecast").is_err());
        assert!(PermissionGate::require_network(&perms, "file:///etc/passwd").is_err());
        assert!(PermissionGate::require_network(&perms, "https://example.org/root").is_err());
    }
}
