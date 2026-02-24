//! Configuration management for Pulse.
//!
//! Handles loading and validating user configuration from YAML files,
//! with support for global and user-specific configs.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use crate::clrs::Clrs;

/// Configuration for a single prompt segment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SegmentConfig {
    /// The name of the segment (e.g., "username", "hostname").
    pub name: String,
    /// Optional color override for this segment.
    /// When not specified, Pulse uses terminal ANSI colors that adapt to your
    /// terminal's configured color palette.
    pub color: Option<String>,
}

/// Main configuration structure for Pulse.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// List of segment configurations.
    pub segments: Vec<SegmentConfig>,
    /// Display mode: "DualLine" or "Inline".
    pub mode: Option<String>,
    /// Cached color lookup for O(1) access.
    #[serde(skip)]
    pub segment_colors: HashMap<String, Clrs>,
}

pub struct ConfigBuilder<State> {
    config: Config,
    _state: PhantomData<State>,
}

pub struct NeedsDefaults;
pub struct HasDefaults;

impl Default for Config {
    fn default() -> Self {
        Self {
            segments: vec![
                SegmentConfig {
                    name: "username".to_string(),
                    color: Some("Blue".to_string()),
                },
                SegmentConfig {
                    name: "hostname".to_string(),
                    color: Some("Green".to_string()),
                },
                SegmentConfig {
                    name: "current_directory".to_string(),
                    color: Some("Silver".to_string()),
                },
                SegmentConfig {
                    name: "git_branch".to_string(),
                    color: Some("Red".to_string()),
                },
            ],
            mode: Some("DualLine".to_string()),
            segment_colors: HashMap::new(),
        }
    }
}

impl Config {
    /// Load configuration from default locations.
    ///
    /// Loads config from the following sources in order of precedence (later sources override earlier):
    /// 1. Default configuration (lowest priority)
    /// 2. Global config at the system config directory (if available)
    /// 3. User config at the platform config directory (highest priority)
    ///
    /// When both the global and user configs define the same segment (by name),
    /// the user config takes precedence - the segment from the user config replaces
    /// the corresponding segment from the global config. Duplicate segments within
    /// a single config file are not supported; only the last occurrence would be kept
    /// when parsed, though this depends on the YAML parser behavior. The display
    /// mode is overridden only when the higher-precedence config explicitly sets it.
    ///
    /// # Preconditions
    /// - The configuration files, if they exist, must be valid YAML.
    /// - Segment names must be one of: "username", "hostname", "current_directory", "git_branch".
    /// - Mode must be one of: "DualLine", "Inline".
    /// - Colors must be valid color names parseable by [`std::str::FromStr`].
    ///
    /// # Postconditions
    /// - Returns a valid `Config` with all segments merged from the applicable sources.
    /// - The returned config has its color cache built for O(1) color lookups.
    ///
    /// # Error Cases
    /// Returns an error if:
    /// - A config file exists but cannot be read.
    /// - A config file contains invalid YAML.
    /// - A config file contains invalid segment names or colors.
    ///
    /// # Example
    /// ```ignore
    /// let config = Config::load().expect("Failed to load config");
    /// let username_color = config.get_color("username");
    /// ```
    pub fn load() -> Result<Self> {
        let mut builder = ConfigBuilder::new().with_defaults();

        // Load global config
        let mut global_path = None;
        for path in system_config_paths() {
            if path.exists() {
                builder = builder.merge_path(&path)?;
                global_path = Some(path);
                break;
            }
        }

        // Load user config
        if let Some(path) = user_config_path()
            && global_path
                .as_ref()
                .is_none_or(|global| global != &path)
        {
            builder = builder.merge_path_if_exists(&path)?;
        }

        Ok(builder.build())
    }

    /// Load configuration from a specific file path.
    ///
    /// Only the provided file is loaded (merged on top of defaults), and
    /// default lookup locations are skipped.
    ///
    /// Returns an error if the explicit file path does not exist.
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let builder = ConfigBuilder::new().with_defaults().merge_path(path)?;
        Ok(builder.build())
    }

    fn build_color_cache(&mut self) {
        self.segment_colors.clear();
        for segment in &self.segments {
            if let Some(color_str) = &segment.color
                && let Ok(color) = color_str.parse::<Clrs>()
            {
                self.segment_colors.insert(segment.name.clone(), color);
            }
        }
    }

    /// Validate the configuration for correctness.
    ///
    /// Checks that all segment names are valid and colors parse correctly.
    pub fn validate(&self) -> Result<()> {
        let valid_names = ["username", "hostname", "current_directory", "git_branch"];
        let valid_modes = ["DualLine", "Inline"];
        for segment in &self.segments {
            if !valid_names.contains(&segment.name.as_str()) {
                return Err(anyhow!("Invalid segment name: {}", segment.name));
            }
            if let Some(color_str) = &segment.color
                && color_str.parse::<Clrs>().is_err()
            {
                return Err(anyhow!("Invalid color: {}", color_str));
            }
        }
        if let Some(mode) = &self.mode
            && !valid_modes.contains(&mode.as_str())
        {
            return Err(anyhow!("Invalid mode: {}", mode));
        }
        Ok(())
    }

    /// Get the color for a given segment name.
    ///
    /// Returns the configured color if available, otherwise defaults.
    /// When no custom color is configured, Pulse uses terminal ANSI colors
    /// that automatically adapt to your terminal's color palette.
    /// On terminals with truecolor support (24-bit), the specific clrs.cc
    /// RGB values are used instead.
    pub fn get_color(&self, name: &str) -> Clrs {
        if let Some(color) = self.segment_colors.get(name) {
            return *color;
        }
        match name {
            "username" => Clrs::Blue,
            "hostname" => Clrs::Green,
            "current_directory" => Clrs::Silver,
            "git_branch" => Clrs::Red,
            _ => Clrs::White,
        }
    }
}

impl ConfigBuilder<NeedsDefaults> {
    pub fn new() -> Self {
        Self {
            config: Config {
                segments: Vec::new(),
                mode: None,
                segment_colors: HashMap::new(),
            },
            _state: PhantomData,
        }
    }

    pub fn with_defaults(self) -> ConfigBuilder<HasDefaults> {
        ConfigBuilder {
            config: Config::default(),
            _state: PhantomData,
        }
    }

}

impl ConfigBuilder<HasDefaults> {
    pub fn merge_path(mut self, path: &Path) -> Result<Self> {
        if !path.exists() {
            return Err(anyhow!("Config file not found: {}", path.display()));
        }
        let loaded_config = read_config_from_path(path)?;
        merge_configs(&mut self.config, loaded_config);
        Ok(self)
    }

    pub fn merge_path_if_exists(mut self, path: &Path) -> Result<Self> {
        if path.exists() {
            let loaded_config = read_config_from_path(path)?;
            merge_configs(&mut self.config, loaded_config);
        }
        Ok(self)
    }

    pub fn build(mut self) -> Config {
        self.config.build_color_cache();
        self.config
    }
}

fn config_file_path(base_dir: PathBuf) -> PathBuf {
    base_dir.join("pulse").join("config.yaml")
}

fn system_config_paths() -> Vec<PathBuf> {
    #[cfg(target_family = "unix")]
    {
        let mut config_dirs = std::env::var_os("XDG_CONFIG_DIRS")
            .map(|dirs| {
                std::env::split_paths(&dirs)
                    .filter(|dir| !dir.as_os_str().is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if config_dirs.is_empty() {
            config_dirs.push(PathBuf::from("/etc/xdg"));
        }

        config_dirs.into_iter().map(config_file_path).collect()
    }
    #[cfg(not(target_family = "unix"))]
    {
        Vec::new()
    }
}

fn user_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(config_file_path)
}

fn merge_configs(config: &mut Config, other: Config) {
    for other_segment in other.segments {
        if let Some(existing) = config
            .segments
            .iter_mut()
            .find(|segment| segment.name == other_segment.name)
        {
            *existing = other_segment;
        } else {
            config.segments.push(other_segment);
        }
    }
    if other.mode.is_some() {
        config.mode = other.mode;
    }
}

fn read_config_from_path(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let loaded_config: Config = serde_yml::from_str(&content)?;
    loaded_config.validate()?;
    Ok(loaded_config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_colors() {
        let config = Config::default();
        assert_eq!(config.get_color("username"), Clrs::Blue);
        assert_eq!(config.get_color("hostname"), Clrs::Green);
        assert_eq!(config.get_color("current_directory"), Clrs::Silver);
        assert_eq!(config.get_color("git_branch"), Clrs::Red);
    }

    #[test]
    fn test_get_color_configured() {
        let mut config = Config::default();
        config.segments[0].color = Some("Red".to_string()); // username
        config.build_color_cache();
        assert_eq!(config.get_color("username"), Clrs::Red);
        // Others should still be default
        assert_eq!(config.get_color("hostname"), Clrs::Green);
    }

    #[test]
    fn test_build_color_cache_clears_removed_override() {
        let mut config = Config::default();
        config.segments[0].color = Some("Red".to_string());
        config.build_color_cache();
        assert_eq!(config.get_color("username"), Clrs::Red);

        config.segments[0].color = None;
        config.build_color_cache();
        assert_eq!(config.get_color("username"), Clrs::Blue);
    }

    #[test]
    fn test_get_color_unknown_segment() {
        let config = Config::default();
        assert_eq!(config.get_color("unknown"), Clrs::White);
    }

    #[test]
    fn test_validate_valid_colors() {
        let config = Config {
            segments: vec![
                SegmentConfig {
                    name: "username".to_string(),
                    color: Some("Blue".to_string()),
                },
                SegmentConfig {
                    name: "hostname".to_string(),
                    color: Some("Green".to_string()),
                },
            ],
            mode: None,
            segment_colors: HashMap::new(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_color() {
        let config = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("InvalidColor".to_string()),
            }],
            mode: None,
            segment_colors: HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_segment_name() {
        let config = Config {
            segments: vec![SegmentConfig {
                name: "invalid_segment".to_string(),
                color: Some("Blue".to_string()),
            }],
            mode: None,
            segment_colors: HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_mode() {
        let config = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("Blue".to_string()),
            }],
            mode: Some("SingleLine".to_string()),
            segment_colors: HashMap::new(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_merge_overrides_existing() {
        let mut base = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("Blue".to_string()),
            }],
            mode: None,
            segment_colors: HashMap::new(),
        };
        let other = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("Red".to_string()),
            }],
            mode: None,
            segment_colors: HashMap::new(),
        };
        merge_configs(&mut base, other);
        base.build_color_cache();
        assert_eq!(base.get_color("username"), Clrs::Red);
    }

    #[test]
    fn test_merge_adds_new_segments() {
        let mut base = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("Blue".to_string()),
            }],
            mode: None,
            segment_colors: HashMap::new(),
        };
        let other = Config {
            segments: vec![SegmentConfig {
                name: "hostname".to_string(),
                color: Some("Green".to_string()),
            }],
            mode: None,
            segment_colors: HashMap::new(),
        };
        merge_configs(&mut base, other);
        base.build_color_cache();
        assert_eq!(base.get_color("username"), Clrs::Blue);
        assert_eq!(base.get_color("hostname"), Clrs::Green);
    }

    #[test]
    fn test_merge_overrides_mode_when_set() {
        let mut base = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("Blue".to_string()),
            }],
            mode: Some("DualLine".to_string()),
            segment_colors: HashMap::new(),
        };
        let other = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("Blue".to_string()),
            }],
            mode: Some("Inline".to_string()),
            segment_colors: HashMap::new(),
        };
        merge_configs(&mut base, other);
        base.build_color_cache();
        assert_eq!(base.mode.as_deref(), Some("Inline"));
    }

    #[test]
    fn test_builder_builds_cache_for_overrides() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        std::io::Write::write_all(
            &mut temp_file,
            b"segments:\n  - name: username\n    color: Red\n",
        )?;

        let config = ConfigBuilder::new()
            .with_defaults()
            .merge_path(temp_file.path())?
            .build();

        assert_eq!(config.get_color("username"), Clrs::Red);
        Ok(())
    }

    #[test]
    fn test_builder_merge_path_if_exists_skips_missing_file() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let missing_path = temp_dir.path().join("missing.yaml");

        let config = ConfigBuilder::new()
            .with_defaults()
            .merge_path_if_exists(&missing_path)?
            .build();

        assert_eq!(config.get_color("username"), Clrs::Blue);
        Ok(())
    }

    #[test]
    fn test_load_from_path_overrides_defaults() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        std::io::Write::write_all(
            &mut temp_file,
            b"segments:\n  - name: username\n    color: Red\n",
        )?;

        let config = Config::load_from_path(temp_file.path())?;

        assert_eq!(config.get_color("username"), Clrs::Red);
        assert_eq!(config.get_color("hostname"), Clrs::Green);
        Ok(())
    }

    #[test]
    fn test_load_from_path_missing_file_returns_error() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let missing_path = temp_dir.path().join("missing.yaml");
        let result = Config::load_from_path(&missing_path);

        assert!(result.is_err());
        Ok(())
    }
}
