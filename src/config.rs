//! Configuration management for Pulse.
//!
//! Handles loading and validating user configuration from YAML files,
//! with support for global and user-specific configs.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::clrs::Clrs;

/// Configuration for a single prompt segment.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SegmentConfig {
    /// The name of the segment (e.g., "username", "hostname").
    pub name: String,
    /// Optional color override for this segment.
    pub color: Option<String>,
}

/// Main configuration structure for Pulse.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// List of segment configurations.
    pub segments: Vec<SegmentConfig>,
    /// Display mode: "DualLine" or "Inline".
    pub mode: Option<String>,
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
        }
    }
}

impl Config {
    /// Load configuration from default locations.
    ///
    /// Loads global config from /etc/pulse/config.yaml, then user config
    /// from ~/.config/pulse/config.yaml, merging them with defaults.
    pub fn load() -> Result<Self> {
        let mut config = Self::default();

        // Load global config
        let global_path = PathBuf::from("/etc/pulse/config.yaml");
        if global_path.exists() {
            let content = std::fs::read_to_string(&global_path)?;
            let global_config: Self = serde_yaml::from_str(&content)?;
            config.validate()?;
            config.merge(global_config);
        }

        // Load user config
        let user_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config")
            .join("pulse")
            .join("config.yaml");
        if user_path.exists() {
            let content = std::fs::read_to_string(&user_path)?;
            let user_config: Self = serde_yaml::from_str(&content)?;
            user_config.validate()?;
            config.merge(user_config);
        }

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
    pub fn get_color(&self, name: &str) -> Clrs {
        for segment in &self.segments {
            if segment.name == name
                && let Some(color_str) = &segment.color
                && let Ok(color) = color_str.parse::<Clrs>()
            {
                return color;
            }
        }
        // Default colors
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
        };
        let other = Config {
            segments: vec![SegmentConfig {
                name: "username".to_string(),
                color: Some("Red".to_string()),
            }],
            mode: None,
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
        };
        let other = Config {
            segments: vec![SegmentConfig {
                name: "hostname".to_string(),
                color: Some("Green".to_string()),
            }],
            mode: None,
        };
        base.merge(other);
        assert_eq!(base.get_color("username"), Clrs::Blue);
        assert_eq!(base.get_color("hostname"), Clrs::Green);
    }
}
