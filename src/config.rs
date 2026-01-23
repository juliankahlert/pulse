use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::clrs::Clrs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SegmentConfig {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub segments: Vec<SegmentConfig>,
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
