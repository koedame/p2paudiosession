//! Audio plugin hosting module
//!
//! Provides support for loading and hosting CLAP and VST3 plugins.

mod clap_host;

pub use clap_host::{ClapPlugin, ClapPluginLoader};

use std::path::Path;

use super::error::AudioError;

/// Plugin format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginFormat {
    /// VST3 plugin format
    Vst3,
    /// CLAP plugin format
    Clap,
    /// AU plugin format (macOS only)
    AudioUnit,
}

/// Plugin parameter information
#[derive(Debug, Clone)]
pub struct PluginParameter {
    pub id: u32,
    pub name: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub unit: String,
}

/// Plugin information
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub format: PluginFormat,
    pub path: String,
    pub uid: String,
    pub num_inputs: u32,
    pub num_outputs: u32,
    pub has_editor: bool,
}

/// Audio plugin trait
pub trait AudioPlugin: Send {
    /// Get plugin information
    fn info(&self) -> &PluginInfo;

    /// Initialize the plugin with sample rate and max block size
    fn initialize(&mut self, sample_rate: f64, max_block_size: u32) -> Result<(), AudioError>;

    /// Activate the plugin for processing
    fn activate(&mut self) -> Result<(), AudioError>;

    /// Deactivate the plugin
    fn deactivate(&mut self);

    /// Process audio samples
    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]);

    /// Get number of parameters
    fn num_parameters(&self) -> usize;

    /// Get parameter info
    fn parameter(&self, index: usize) -> Option<PluginParameter>;

    /// Set parameter value
    fn set_parameter(&mut self, index: usize, value: f32);

    /// Get parameter value
    fn get_parameter(&self, index: usize) -> f32;

    /// Open plugin editor window
    fn open_editor(&mut self, parent: *mut std::ffi::c_void) -> Result<(), AudioError>;

    /// Close plugin editor window
    fn close_editor(&mut self);

    /// Check if editor is open
    fn is_editor_open(&self) -> bool;
}

/// Plugin scanner for finding installed plugins
pub struct PluginScanner {
    search_paths: Vec<String>,
}

impl PluginScanner {
    /// Create a new plugin scanner with default search paths
    pub fn new() -> Self {
        Self {
            search_paths: default_plugin_paths(),
        }
    }

    /// Add a custom search path
    pub fn add_search_path(&mut self, path: &str) {
        self.search_paths.push(path.to_string());
    }

    /// Scan for plugins and return their info
    pub fn scan(&self) -> Vec<PluginInfo> {
        let mut plugins = Vec::new();

        for path in &self.search_paths {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(info) = self.scan_plugin(&path) {
                        plugins.push(info);
                    }
                }
            }
        }

        plugins
    }

    /// Scan a single plugin file/bundle
    fn scan_plugin(&self, path: &Path) -> Option<PluginInfo> {
        let extension = path.extension()?.to_str()?;

        let format = match extension {
            "vst3" => PluginFormat::Vst3,
            "clap" => PluginFormat::Clap,
            "component" => PluginFormat::AudioUnit,
            #[cfg(target_os = "linux")]
            "so" => {
                // Check if it's in a CLAP or VST3 directory
                let path_str = path.to_string_lossy();
                if path_str.contains("clap") || path_str.contains("CLAP") {
                    PluginFormat::Clap
                } else if path_str.contains("vst3") || path_str.contains("VST3") {
                    PluginFormat::Vst3
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        let name = path.file_stem()?.to_str()?.to_string();

        // Try to load and get actual plugin info for CLAP plugins
        if format == PluginFormat::Clap {
            if let Ok(loader) = ClapPluginLoader::new(path.to_string_lossy().as_ref()) {
                if let Some(desc) = loader.get_plugin_descriptor(0) {
                    return Some(desc);
                }
            }
        }

        // Fallback to basic info
        Some(PluginInfo {
            name: name.clone(),
            vendor: "Unknown".to_string(),
            version: "1.0.0".to_string(),
            format,
            path: path.to_string_lossy().to_string(),
            uid: name,
            num_inputs: 2,
            num_outputs: 2,
            has_editor: true,
        })
    }
}

impl Default for PluginScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Get default plugin search paths for the current platform
fn default_plugin_paths() -> Vec<String> {
    let mut paths = Vec::new();

    #[cfg(target_os = "linux")]
    {
        // VST3
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/.vst3", home));
        }
        paths.push("/usr/lib/vst3".to_string());
        paths.push("/usr/local/lib/vst3".to_string());

        // CLAP
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/.clap", home));
        }
        paths.push("/usr/lib/clap".to_string());
        paths.push("/usr/local/lib/clap".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        // VST3
        paths.push("/Library/Audio/Plug-Ins/VST3".to_string());
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/Library/Audio/Plug-Ins/VST3", home));
        }

        // CLAP
        paths.push("/Library/Audio/Plug-Ins/CLAP".to_string());
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/Library/Audio/Plug-Ins/CLAP", home));
        }

        // AU
        paths.push("/Library/Audio/Plug-Ins/Components".to_string());
        if let Ok(home) = std::env::var("HOME") {
            paths.push(format!("{}/Library/Audio/Plug-Ins/Components", home));
        }
    }

    #[cfg(target_os = "windows")]
    {
        // VST3
        paths.push("C:\\Program Files\\Common Files\\VST3".to_string());
        paths.push("C:\\Program Files (x86)\\Common Files\\VST3".to_string());

        // CLAP
        paths.push("C:\\Program Files\\Common Files\\CLAP".to_string());
        paths.push("C:\\Program Files (x86)\\Common Files\\CLAP".to_string());
    }

    paths
}

/// Plugin host for managing loaded plugins
pub struct PluginHost {
    plugins: Vec<Box<dyn AudioPlugin>>,
    sample_rate: f64,
    block_size: u32,
}

impl PluginHost {
    /// Create a new plugin host
    pub fn new(sample_rate: f64, block_size: u32) -> Self {
        Self {
            plugins: Vec::new(),
            sample_rate,
            block_size,
        }
    }

    /// Load a plugin from path
    pub fn load_plugin(&mut self, path: &str) -> Result<usize, AudioError> {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let plugin: Box<dyn AudioPlugin> = match extension {
            "clap" | "so" => {
                let loader = ClapPluginLoader::new(path)
                    .map_err(|e| AudioError::PluginError(e.to_string()))?;
                let mut plugin = loader
                    .instantiate(0)
                    .map_err(|e| AudioError::PluginError(e.to_string()))?;
                plugin.initialize(self.sample_rate, self.block_size)?;
                plugin.activate()?;
                Box::new(plugin)
            }
            "vst3" => {
                return Err(AudioError::PluginError(
                    "VST3 plugin loading not yet implemented".to_string(),
                ));
            }
            _ => {
                return Err(AudioError::PluginError(format!(
                    "Unknown plugin format: {}",
                    extension
                )));
            }
        };

        let index = self.plugins.len();
        self.plugins.push(plugin);
        Ok(index)
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, index: usize) -> Result<(), AudioError> {
        if index >= self.plugins.len() {
            return Err(AudioError::PluginError("Plugin not found".to_string()));
        }
        self.plugins[index].deactivate();
        self.plugins.remove(index);
        Ok(())
    }

    /// Process audio through all plugins
    pub fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]) {
        for plugin in &mut self.plugins {
            plugin.process(inputs, outputs);
        }
    }

    /// Get number of loaded plugins
    pub fn num_plugins(&self) -> usize {
        self.plugins.len()
    }

    /// Get plugin info
    pub fn plugin_info(&self, index: usize) -> Option<&PluginInfo> {
        self.plugins.get(index).map(|p| p.info())
    }

    /// Get mutable reference to plugin
    pub fn plugin_mut(&mut self, index: usize) -> Option<&mut Box<dyn AudioPlugin>> {
        self.plugins.get_mut(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_scanner_creation() {
        let scanner = PluginScanner::new();
        let _plugins = scanner.scan();
    }

    #[test]
    fn test_plugin_host_creation() {
        let host = PluginHost::new(48000.0, 512);
        assert_eq!(host.num_plugins(), 0);
    }

    #[test]
    fn test_default_plugin_paths() {
        let paths = default_plugin_paths();
        assert!(!paths.is_empty());
    }
}
