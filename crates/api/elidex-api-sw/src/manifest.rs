//! Web App Manifest parser (W3C Web App Manifest).

use serde::{Deserialize, Serialize};

/// Parsed Web App Manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebAppManifest {
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub start_url: Option<String>,
    pub scope: Option<String>,
    /// Stable app identity (URL format).
    pub id: Option<String>,
    #[serde(default)]
    pub display: DisplayMode,
    /// Array of display modes; browser chooses first supported (newer spec addition).
    #[serde(default)]
    pub display_override: Vec<DisplayMode>,
    pub orientation: Option<String>,
    pub theme_color: Option<String>,
    pub background_color: Option<String>,
    #[serde(default)]
    pub icons: Vec<ManifestIcon>,
    #[serde(default)]
    pub shortcuts: Vec<ManifestShortcut>,
    #[serde(default)]
    pub categories: Vec<String>,
    pub lang: Option<String>,
    /// Text direction: "ltr", "rtl", or "auto".
    pub dir: Option<String>,
}

/// Display mode for the PWA.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayMode {
    Fullscreen,
    Standalone,
    MinimalUi,
    #[default]
    Browser,
}

/// An icon declaration in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestIcon {
    pub src: String,
    pub sizes: Option<String>,
    #[serde(rename = "type")]
    pub icon_type: Option<String>,
    /// "any", "maskable", "monochrome".
    pub purpose: Option<String>,
}

/// App shortcut (jump list item).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestShortcut {
    pub name: String,
    pub short_name: Option<String>,
    pub description: Option<String>,
    pub url: String,
    #[serde(default)]
    pub icons: Vec<ManifestIcon>,
}

/// Parse a manifest JSON string.
///
/// Per spec: invalid JSON → all members use defaults (no error).
/// Unknown members are ignored (future compatibility).
pub fn parse_manifest(json: &str) -> WebAppManifest {
    serde_json::from_str(json).unwrap_or_default()
}

/// Resolve relative URLs in a manifest against the manifest URL.
///
/// Per spec: base URL for resolution is the manifest URL, not the page URL.
pub fn resolve_urls(manifest: &mut WebAppManifest, manifest_url: &url::Url) {
    if let Some(ref start) = manifest.start_url {
        if let Ok(resolved) = manifest_url.join(start) {
            manifest.start_url = Some(resolved.to_string());
        }
    }
    if let Some(ref scope) = manifest.scope {
        if let Ok(resolved) = manifest_url.join(scope) {
            manifest.scope = Some(resolved.to_string());
        }
    }
    for icon in &mut manifest.icons {
        if let Ok(resolved) = manifest_url.join(&icon.src) {
            icon.src = resolved.to_string();
        }
    }
    for shortcut in &mut manifest.shortcuts {
        if let Ok(resolved) = manifest_url.join(&shortcut.url) {
            shortcut.url = resolved.to_string();
        }
        for icon in &mut shortcut.icons {
            if let Ok(resolved) = manifest_url.join(&icon.src) {
                icon.src = resolved.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_manifest() {
        let json = "{\
            \"name\": \"My App\",\
            \"short_name\": \"App\",\
            \"description\": \"A test application\",\
            \"start_url\": \"/\",\
            \"scope\": \"/\",\
            \"id\": \"https://example.com/app\",\
            \"display\": \"standalone\",\
            \"display_override\": [\"standalone\", \"minimal-ui\"],\
            \"orientation\": \"portrait\",\
            \"theme_color\": \"#ff0000\",\
            \"background_color\": \"#ffffff\",\
            \"icons\": [{\"src\": \"/icon.png\", \"sizes\": \"192x192\", \"type\": \"image/png\"}],\
            \"shortcuts\": [{\"name\": \"New\", \"url\": \"/new\", \"icons\": []}],\
            \"categories\": [\"productivity\"],\
            \"lang\": \"en\",\
            \"dir\": \"ltr\"\
        }";

        let m = parse_manifest(json);
        assert_eq!(m.name.as_deref(), Some("My App"));
        assert_eq!(m.short_name.as_deref(), Some("App"));
        assert_eq!(m.display, DisplayMode::Standalone);
        assert_eq!(m.display_override.len(), 2);
        assert_eq!(m.icons.len(), 1);
        assert_eq!(m.shortcuts.len(), 1);
        assert_eq!(m.categories, vec!["productivity"]);
    }

    #[test]
    fn parse_minimal_manifest() {
        let m = parse_manifest(r#"{"name": "Test"}"#);
        assert_eq!(m.name.as_deref(), Some("Test"));
        assert_eq!(m.display, DisplayMode::Browser); // default
        assert!(m.icons.is_empty());
    }

    #[test]
    fn parse_invalid_json_returns_defaults() {
        let m = parse_manifest("not json at all");
        assert!(m.name.is_none());
        assert_eq!(m.display, DisplayMode::Browser);
    }

    #[test]
    fn parse_empty_object() {
        let m = parse_manifest("{}");
        assert!(m.name.is_none());
        assert!(m.start_url.is_none());
    }

    #[test]
    fn resolve_relative_urls() {
        let mut m = parse_manifest(
            r#"{
            "start_url": "/app/",
            "scope": "/app/",
            "icons": [{ "src": "icon.png" }],
            "shortcuts": [{ "name": "Home", "url": ".", "icons": [{ "src": "s.png" }] }]
        }"#,
        );

        let manifest_url = url::Url::parse("https://example.com/manifests/app.json").unwrap();
        resolve_urls(&mut m, &manifest_url);

        assert_eq!(m.start_url.as_deref(), Some("https://example.com/app/"));
        assert_eq!(m.scope.as_deref(), Some("https://example.com/app/"));
        assert_eq!(m.icons[0].src, "https://example.com/manifests/icon.png");
        assert_eq!(m.shortcuts[0].url, "https://example.com/manifests/");
        assert_eq!(
            m.shortcuts[0].icons[0].src,
            "https://example.com/manifests/s.png"
        );
    }

    #[test]
    fn display_mode_serde() {
        assert_eq!(
            serde_json::from_str::<DisplayMode>(r#""standalone""#).unwrap(),
            DisplayMode::Standalone
        );
        assert_eq!(
            serde_json::from_str::<DisplayMode>(r#""minimal-ui""#).unwrap(),
            DisplayMode::MinimalUi
        );
        assert_eq!(
            serde_json::from_str::<DisplayMode>(r#""fullscreen""#).unwrap(),
            DisplayMode::Fullscreen
        );
    }
}
