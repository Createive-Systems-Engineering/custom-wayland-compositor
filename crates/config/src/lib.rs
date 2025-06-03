//! Configuration management for the custom Wayland compositor
//! 
//! This crate provides hot-reloadable configuration management with support for
//! multiple formats (TOML, RON) and environment-based overrides. It's designed
//! for 4K displays with DPI-aware defaults and glassmorphism/neomorphism theming.

use anyhow::{Context, Result};
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info};
use compositor_utils::error::CompositorError;

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("TOML parsing error: {0}")]
    TomlParsing(#[from] toml::de::Error),
    
    #[error("RON parsing error: {0}")]
    RonParsing(#[from] ron::error::SpannedError),
    
    #[error("File watching error: {0}")]
    Watcher(#[from] notify::Error),
    
    #[error("Configuration validation error: {message}")]
    Validation { message: String },
    
    #[error("Environment override error: {0}")]
    Environment(String),
}

impl From<ConfigError> for CompositorError {
    fn from(err: ConfigError) -> Self {
        CompositorError::configuration(err.to_string())
    }
}

/// Display configuration for 4K optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    /// Target resolution (width, height)
    pub resolution: (u32, u32),
    /// DPI scaling factor
    pub scale_factor: f64,
    /// Refresh rate in Hz
    pub refresh_rate: u32,
    /// Enable VSync
    pub vsync: bool,
    /// Enable adaptive sync (FreeSync/G-Sync)
    pub adaptive_sync: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            resolution: (3840, 2160), // 4K default
            scale_factor: 2.0,         // 2x scaling for 4K
            refresh_rate: 60,
            vsync: true,
            adaptive_sync: true,
        }
    }
}

/// Theme configuration for glassmorphism/neomorphism
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    /// Theme name
    pub name: String,
    /// Primary color (RGBA)
    pub primary_color: [f32; 4],
    /// Secondary color (RGBA)
    pub secondary_color: [f32; 4],
    /// Accent color (RGBA)
    pub accent_color: [f32; 4],
    /// Background color (RGBA)
    pub background_color: [f32; 4],
    /// Corner radius for elements
    pub corner_radius: f32,
    /// Shadow intensity
    pub shadow_intensity: f32,
    /// Enable animations
    pub animations: bool,
    /// Animation duration in milliseconds
    pub animation_duration: u64,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "glassmorphism".to_string(),
            primary_color: [0.2, 0.2, 0.2, 0.8],     // Semi-transparent dark
            secondary_color: [0.3, 0.3, 0.3, 0.6],   // Lighter semi-transparent
            accent_color: [0.0, 0.5, 1.0, 1.0],      // Blue accent
            background_color: [0.05, 0.05, 0.05, 0.9], // Almost black with transparency
            corner_radius: 12.0,
            shadow_intensity: 0.3,
            animations: true,
            animation_duration: 250,
        }
    }
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable GPU acceleration
    pub gpu_acceleration: bool,
    /// Vulkan device preference: "discrete", "integrated", "any"
    pub vulkan_device_preference: String,
    /// Maximum frame rate
    pub max_fps: u32,
    /// Enable frame rate limiting
    pub frame_limiting: bool,
    /// Memory pool size in MB
    pub memory_pool_size: u64,
    /// Enable performance profiling
    pub profiling: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            gpu_acceleration: true,
            vulkan_device_preference: "discrete".to_string(),
            max_fps: 120,
            frame_limiting: true,
            memory_pool_size: 512, // 512MB
            profiling: false,
        }
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin directory path
    pub plugin_dir: PathBuf,
    /// Auto-load plugins on startup
    pub auto_load: bool,
    /// Hot-reload plugins on file changes
    pub hot_reload: bool,
    /// Enabled plugins list
    pub enabled_plugins: Vec<String>,
    /// Plugin-specific configuration
    pub plugin_settings: std::collections::HashMap<String, toml::Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugin_dir: dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("/etc"))
                .join("custom-compositor")
                .join("plugins"),
            auto_load: true,
            hot_reload: true,
            enabled_plugins: vec![],
            plugin_settings: std::collections::HashMap::new(),
        }
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorConfig {
    /// Display configuration
    pub display: DisplayConfig,
    /// Theme configuration
    pub theme: ThemeConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Plugin configuration
    pub plugins: PluginConfig,
}

impl Default for CompositorConfig {
    fn default() -> Self {
        Self {
            display: DisplayConfig::default(),
            theme: ThemeConfig::default(),
            performance: PerformanceConfig::default(),
            plugins: PluginConfig::default(),
        }
    }
}

impl CompositorConfig {
    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate display configuration
        if self.display.scale_factor <= 0.0 {
            return Err(ConfigError::Validation {
                message: "Display scale factor must be positive".to_string(),
            });
        }
        
        if self.display.refresh_rate == 0 {
            return Err(ConfigError::Validation {
                message: "Display refresh rate must be positive".to_string(),
            });
        }
        
        // Validate theme colors (RGBA values should be 0.0-1.0)
        for color in [
            &self.theme.primary_color,
            &self.theme.secondary_color,
            &self.theme.accent_color,
            &self.theme.background_color,
        ] {
            for &component in color {
                if !(0.0..=1.0).contains(&component) {
                    return Err(ConfigError::Validation {
                        message: "Color components must be between 0.0 and 1.0".to_string(),
                    });
                }
            }
        }
        
        // Validate performance configuration
        if self.performance.max_fps == 0 {
            return Err(ConfigError::Validation {
                message: "Maximum FPS must be positive".to_string(),
            });
        }
        
        Ok(())
    }
    
    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        // Display overrides
        if let Ok(resolution) = std::env::var("COMPOSITOR_RESOLUTION") {
            let parts: Vec<&str> = resolution.split('x').collect();
            if parts.len() == 2 {
                self.display.resolution = (
                    parts[0].parse().map_err(|_| ConfigError::Environment(
                        "Invalid resolution width".to_string()
                    ))?,
                    parts[1].parse().map_err(|_| ConfigError::Environment(
                        "Invalid resolution height".to_string()
                    ))?,
                );
            }
        }
        
        if let Ok(scale) = std::env::var("COMPOSITOR_SCALE") {
            self.display.scale_factor = scale.parse().map_err(|_| {
                ConfigError::Environment("Invalid scale factor".to_string())
            })?;
        }
        
        // Performance overrides
        if let Ok(gpu) = std::env::var("COMPOSITOR_GPU_ACCELERATION") {
            self.performance.gpu_acceleration = gpu.parse().unwrap_or(true);
        }
        
        if let Ok(device) = std::env::var("COMPOSITOR_VULKAN_DEVICE") {
            self.performance.vulkan_device_preference = device;
        }
        
        Ok(())
    }
}

/// Configuration manager with hot-reloading support
pub struct ConfigManager {
    config: Arc<RwLock<CompositorConfig>>,
    config_path: PathBuf,
    _watcher: Option<RecommendedWatcher>,
    change_sender: broadcast::Sender<CompositorConfig>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub async fn new(config_path: Option<PathBuf>) -> Result<Self> {
        let config_path = config_path.unwrap_or_else(|| {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("/etc"))
                .join("custom-compositor")
                .join("config.toml")
        });
        
        // Load or create default configuration
        let config = if config_path.exists() {
            Self::load_config(&config_path).await?
        } else {
            let default_config = CompositorConfig::default();
            Self::save_config(&config_path, &default_config).await?;
            default_config
        };
        
        let (change_sender, _) = broadcast::channel(32);
        
        let config_manager = Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
            _watcher: None,
            change_sender,
        };
        
        info!("Configuration manager initialized");
        Ok(config_manager)
    }
    
    /// Get current configuration
    pub async fn get_config(&self) -> CompositorConfig {
        self.config.read().await.clone()
    }
    
    /// Update configuration
    pub async fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut CompositorConfig),
    {
        let mut config = self.config.write().await;
        updater(&mut config);
        
        // Validate updated configuration
        config.validate()?;
        
        // Save to file
        Self::save_config(&self.config_path, &config).await?;
        
        // Notify subscribers of changes
        let _ = self.change_sender.send(config.clone());
        
        info!("Configuration updated");
        Ok(())
    }
    
    /// Subscribe to configuration changes
    pub fn subscribe_to_changes(&self) -> broadcast::Receiver<CompositorConfig> {
        self.change_sender.subscribe()
    }
    
    /// Reload configuration from file
    pub async fn reload(&self) -> Result<()> {
        let mut config = Self::load_config(&self.config_path).await?;
        config.apply_env_overrides()?;
        config.validate()?;
        
        *self.config.write().await = config.clone();
        let _ = self.change_sender.send(config);
        
        info!("Configuration reloaded from file");
        Ok(())
    }
    
    /// Load configuration from file
    async fn load_config(path: &Path) -> Result<CompositorConfig> {
        let content = tokio::fs::read_to_string(path)
            .await
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let mut config: CompositorConfig = if path.extension() == Some("ron".as_ref()) {
            ron::from_str(&content)
                .with_context(|| "Failed to parse RON configuration")?
        } else {
            toml::from_str(&content)
                .with_context(|| "Failed to parse TOML configuration")?
        };
        
        // Apply environment overrides
        config.apply_env_overrides()?;
        
        debug!("Configuration loaded from {}", path.display());
        Ok(config)
    }
    
    /// Save configuration to file
    async fn save_config(path: &Path, config: &CompositorConfig) -> Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = if path.extension() == Some("ron".as_ref()) {
            ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::default())
                .with_context(|| "Failed to serialize configuration to RON")?
        } else {
            toml::to_string_pretty(config)
                .with_context(|| "Failed to serialize configuration to TOML")?
        };
        
        tokio::fs::write(path, content)
            .await
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        
        debug!("Configuration saved to {}", path.display());
        Ok(())
    }
    
    /// Enable hot-reloading of configuration files
    pub async fn enable_hot_reload(&mut self) -> Result<()> {
        let _config_path = self.config_path.clone();
        let _config = self.config.clone();
        let _sender = self.change_sender.clone();
        
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        if event.kind.is_modify() {
                            debug!("Configuration file changed, reloading...");
                            // Note: In a real implementation, we'd need to handle this
                            // in an async context. For now, we'll just log it.
                            info!("Configuration file modified");
                        }
                    }
                    Err(e) => error!("File watcher error: {}", e),
                }
            },
            NotifyConfig::default(),
        )?;
        
        watcher.watch(&self.config_path, RecursiveMode::NonRecursive)?;
        self._watcher = Some(watcher);
        
        info!("Hot-reload enabled for configuration");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_default_config_validation() {
        let config = CompositorConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[tokio::test]
    async fn test_config_serialization() {
        let config = CompositorConfig::default();
        
        // Test TOML serialization
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: CompositorConfig = toml::from_str(&toml_str).unwrap();
        assert!(deserialized.validate().is_ok());
    }
    
    #[tokio::test]
    async fn test_config_manager() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        let manager = ConfigManager::new(Some(config_path.clone())).await.unwrap();
        let config = manager.get_config().await;
        
        assert_eq!(config.display.resolution, (3840, 2160));
        assert!(config_path.exists());
    }
    
    #[tokio::test]
    async fn test_env_overrides() {
        std::env::set_var("COMPOSITOR_RESOLUTION", "1920x1080");
        std::env::set_var("COMPOSITOR_SCALE", "1.5");
        
        let mut config = CompositorConfig::default();
        config.apply_env_overrides().unwrap();
        
        assert_eq!(config.display.resolution, (1920, 1080));
        assert_eq!(config.display.scale_factor, 1.5);
        
        std::env::remove_var("COMPOSITOR_RESOLUTION");
        std::env::remove_var("COMPOSITOR_SCALE");
    }
}
