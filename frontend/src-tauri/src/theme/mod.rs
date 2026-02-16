use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThemeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Theme not found: {0}")]
    NotFound(String),
    #[error("Invalid theme: {0}")]
    Invalid(String),
    #[allow(dead_code)]
    #[error("Network error: {0}")]
    NetworkError(String),
}

/// A single theme with its metadata and preview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    /// Path to the theme directory on disk (once installed)
    pub path: Option<String>,
    /// Relative path to the preview thumbnail image
    pub thumbnail: Option<String>,
    /// Relative path to the full-size screenshot
    pub screenshot: Option<String>,
    /// Whether this theme is currently installed locally
    pub installed: bool,
    /// Whether this is the currently active theme
    pub active: bool,
    /// Theme style: "graphical" (BURG-like) or "text" (classic text menu)
    pub style: ThemeStyle,
    /// URL to download the theme from repo
    pub repo_url: Option<String>,
    /// Base64-encoded thumbnail for quick preview (small ~5KB image)
    pub thumbnail_data: Option<String>,
}

/// Theme style — text-based or graphical (like BURG used to be)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeStyle {
    Text,
    Graphical,
}

/// Theme repository index (fetched from remote)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeIndex {
    pub version: u32,
    pub themes: Vec<ThemeManifest>,
}

/// A theme entry from the remote repository
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub style: ThemeStyle,
    pub download_url: String,
    pub thumbnail_url: String,
    pub screenshot_url: Option<String>,
}

/// Manages themes: local installation, remote repo browsing, preview generation
pub struct ThemeManager {
    themes_dir: PathBuf,
    #[allow(dead_code)]
    cache_dir: PathBuf,
}

impl ThemeManager {
    pub fn new(base_dir: &Path) -> Result<Self, ThemeError> {
        let themes_dir = base_dir.join("themes");
        let cache_dir = base_dir.join("cache").join("themes");
        fs::create_dir_all(&themes_dir)?;
        fs::create_dir_all(&cache_dir)?;

        Ok(Self {
            themes_dir,
            cache_dir,
        })
    }

    /// List all locally installed themes
    pub fn list_installed(&self) -> Result<Vec<Theme>, ThemeError> {
        let mut themes = Vec::new();

        if !self.themes_dir.exists() {
            return Ok(themes);
        }

        for entry in fs::read_dir(&self.themes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Ok(theme) = self.load_theme_from_dir(&path) {
                    themes.push(theme);
                }
            }
        }

        // Sort by name
        themes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(themes)
    }

    /// Load a theme from its directory (reads theme.toml manifest)
    fn load_theme_from_dir(&self, dir: &Path) -> Result<Theme, ThemeError> {
        let manifest_path = dir.join("theme.toml");
        if !manifest_path.exists() {
            return Err(ThemeError::Invalid(format!(
                "No theme.toml found in {}",
                dir.display()
            )));
        }

        let content = fs::read_to_string(&manifest_path)?;
        let manifest: toml::Value = content
            .parse()
            .map_err(|e| ThemeError::Invalid(format!("Invalid TOML: {}", e)))?;

        let id = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let name = manifest
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        let description = manifest
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let author = manifest
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();

        let version = manifest
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();

        let style = manifest
            .get("style")
            .and_then(|v| v.as_str())
            .map(|s| match s {
                "graphical" => ThemeStyle::Graphical,
                _ => ThemeStyle::Text,
            })
            .unwrap_or(ThemeStyle::Text);

        // Load thumbnail as base64 for quick preview
        let thumbnail_path = dir.join("thumbnail.png");
        let thumbnail = if thumbnail_path.exists() {
            Some(thumbnail_path.to_string_lossy().to_string())
        } else {
            None
        };

        let thumbnail_data = if thumbnail_path.exists() {
            use std::io::Read;
            let mut file = fs::File::open(&thumbnail_path)?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)?;
            Some(format!(
                "data:image/png;base64,{}",
                base64_encode(&buf)
            ))
        } else {
            None
        };

        let screenshot_path = dir.join("screenshot.png");
        let screenshot = if screenshot_path.exists() {
            Some(screenshot_path.to_string_lossy().to_string())
        } else {
            None
        };

        // Check if this is the active theme
        let active = self.get_active_theme_id()
            .map(|active_id| active_id == id)
            .unwrap_or(false);

        Ok(Theme {
            id,
            name,
            description,
            author,
            version,
            path: Some(dir.to_string_lossy().to_string()),
            thumbnail,
            screenshot,
            installed: true,
            active,
            style,
            repo_url: None,
            thumbnail_data,
        })
    }

    /// Get the currently active theme ID
    fn get_active_theme_id(&self) -> Option<String> {
        let config_path = self.themes_dir.parent()?.join("config").join("settings.toml");
        let content = fs::read_to_string(config_path).ok()?;
        let config: toml::Value = content.parse().ok()?;
        config
            .get("theme")
            .and_then(|t| t.get("active"))
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// Set the active theme
    pub fn set_active_theme(&self, theme_id: &str) -> Result<(), ThemeError> {
        let theme_dir = self.themes_dir.join(theme_id);
        if !theme_dir.exists() {
            return Err(ThemeError::NotFound(theme_id.to_string()));
        }

        let config_dir = self.themes_dir.parent()
            .ok_or_else(|| ThemeError::Invalid("No parent dir".into()))?
            .join("config");
        fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("settings.toml");
        let mut config = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            content
                .parse::<toml::Value>()
                .unwrap_or(toml::Value::Table(toml::map::Map::new()))
        } else {
            toml::Value::Table(toml::map::Map::new())
        };

        // Set theme.active
        if let Some(table) = config.as_table_mut() {
            let theme_table = table
                .entry("theme")
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
            if let Some(t) = theme_table.as_table_mut() {
                t.insert(
                    "active".to_string(),
                    toml::Value::String(theme_id.to_string()),
                );
            }
        }

        fs::write(&config_path, toml::to_string_pretty(&config)
            .map_err(|e| ThemeError::Invalid(e.to_string()))?)?;

        Ok(())
    }

    /// Get a theme's thumbnail data (base64 encoded for frontend display)
    pub fn get_thumbnail_data(&self, theme_id: &str) -> Result<Option<String>, ThemeError> {
        let thumbnail_path = self.themes_dir.join(theme_id).join("thumbnail.png");
        if !thumbnail_path.exists() {
            return Ok(None);
        }

        let data = fs::read(&thumbnail_path)?;
        Ok(Some(format!(
            "data:image/png;base64,{}",
            base64_encode(&data)
        )))
    }

    /// Get a theme's full screenshot data
    pub fn get_screenshot_data(&self, theme_id: &str) -> Result<Option<String>, ThemeError> {
        let screenshot_path = self.themes_dir.join(theme_id).join("screenshot.png");
        if !screenshot_path.exists() {
            return Ok(None);
        }

        let data = fs::read(&screenshot_path)?;
        Ok(Some(format!(
            "data:image/png;base64,{}",
            base64_encode(&data)
        )))
    }

    /// Install a theme from a directory (e.g., downloaded/extracted)
    #[allow(dead_code)]
    pub fn install_from_dir(&self, source: &Path) -> Result<Theme, ThemeError> {
        let manifest_path = source.join("theme.toml");
        if !manifest_path.exists() {
            return Err(ThemeError::Invalid(
                "Source directory missing theme.toml".into(),
            ));
        }

        // Read the manifest to get the theme ID
        let content = fs::read_to_string(&manifest_path)?;
        let manifest: toml::Value = content
            .parse()
            .map_err(|e| ThemeError::Invalid(format!("Invalid TOML: {}", e)))?;

        let id = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                manifest
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string()
            });

        let dest = self.themes_dir.join(&id);

        // Copy the directory
        copy_dir_recursive(source, &dest)?;

        self.load_theme_from_dir(&dest)
    }

    /// Uninstall a theme
    #[allow(dead_code)]
    pub fn uninstall(&self, theme_id: &str) -> Result<(), ThemeError> {
        let theme_dir = self.themes_dir.join(theme_id);
        if !theme_dir.exists() {
            return Err(ThemeError::NotFound(theme_id.to_string()));
        }

        // Don't allow uninstalling the active theme
        if let Some(active) = self.get_active_theme_id() {
            if active == theme_id {
                return Err(ThemeError::Invalid(
                    "Cannot uninstall the currently active theme. Switch to another theme first."
                        .into(),
                ));
            }
        }

        fs::remove_dir_all(&theme_dir)?;
        Ok(())
    }

    /// Create a default theme for fresh installations
    pub fn create_default_themes(&self) -> Result<(), ThemeError> {
        // Create "classic" text theme
        let classic_dir = self.themes_dir.join("classic");
        if !classic_dir.exists() {
            fs::create_dir_all(&classic_dir)?;
            fs::write(
                classic_dir.join("theme.toml"),
                r#"name = "Classic"
description = "Clean text-based boot menu with distro detection"
author = "unix-lvm-loader"
version = "1.0.0"
style = "text"
"#,
            )?;
        }

        // Create "graphical" BURG-style theme
        let graphical_dir = self.themes_dir.join("modern");
        if !graphical_dir.exists() {
            fs::create_dir_all(&graphical_dir)?;
            fs::write(
                graphical_dir.join("theme.toml"),
                r#"name = "Modern"
description = "Graphical boot menu with OS icons and animations (BURG-style)"
author = "unix-lvm-loader"
version = "1.0.0"
style = "graphical"
"#,
            )?;
        }

        // Set classic as default if no active theme
        if self.get_active_theme_id().is_none() {
            self.set_active_theme("classic")?;
        }

        Ok(())
    }

    /// Get themes directory
    #[allow(dead_code)]
    pub fn themes_dir(&self) -> &Path {
        &self.themes_dir
    }

    /// Get cache directory
    #[allow(dead_code)]
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

/// Simple base64 encoder (avoids adding base64 crate dependency)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Recursively copy a directory
#[allow(dead_code)]
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ThemeError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
