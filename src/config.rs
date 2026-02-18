//! Configuration management for Pulse.
//!
//! Handles loading and validating user configuration from YAML files,
//! with support for global and user-specific configs.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
    /// 2. Global config at `/etc/pulse/config.yaml`
    /// 3. User config at `~/.config/pulse/config.yaml` (highest priority)
    ///
    /// When both the global and user configs define the same segment (by name),
    /// the user config takes precedence - the segment from the user config replaces
    /// the corresponding segment from the global config. Duplicate segments within
    /// a single config file are not supported; only the last occurrence would be kept
    /// when parsed, though this depends on the YAML parser behavior.
    ///
    /// # Preconditions
    /// - The configuration files, if they exist, must be valid YAML.
    /// - Segment names must be one of: "username", "hostname", "current_directory", "git_branch".
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
        let mut config = Self::default();

        // Load global config
        let global_path = PathBuf::from("/etc/pulse/config.yaml");
        if global_path.exists() {
            let content = std::fs::read_to_string(&global_path)?;
            let global_config: Self = serde_yml::from_str(&content)?;
            config.validate()?;
            config.merge(global_config);
        }

        // Load user config
        let user_path = dirs::home_dir()
            .ok_or_else(|| anyhow!("Cannot determine home directory"))?
            .join(".config")
            .join("pulse")
            .join("config.yaml");
        if user_path.exists() {
            let content = std::fs::read_to_string(&user_path)?;
            let user_config: Self = serde_yml::from_str(&content)?;
            user_config.validate()?;
            config.merge(user_config);
        }

        config.build_color_cache();
        Ok(config)
    }

    /// Merge another config into this one, overriding existing segments.
    fn merge(&mut self, other: Self) {
        for other_segment in other.segments {
            if let Some(existing) = self
                .segments
                .iter_mut()
                .find(|s| s.name == other_segment.name)
            {
                *existing = other_segment;
            } else {
                self.segments.push(other_segment);
            }
        }
        self.build_color_cache();
    }

    fn build_color_cache(&mut self) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
        base.merge(other);
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
        base.merge(other);
        assert_eq!(base.get_color("username"), Clrs::Blue);
        assert_eq!(base.get_color("hostname"), Clrs::Green);
    }
}
