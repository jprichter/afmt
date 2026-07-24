use crate::formatter::Config;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_INCLUDES: &[&str] = &["**/*.cls", "**/*.trigger", "**/*.apex", "**/*.apexc"];
const DEFAULT_EXCLUDES: &[&str] = &["**/.git/**", "**/.sfdx/**", "**/node_modules/**"];

#[derive(Clone, Debug, Default, Deserialize)]
pub struct AfmtConfig {
    #[serde(flatten)]
    pub formatter: Config,
    #[serde(default)]
    pub files: FileSelectionConfig,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct FileSelectionConfig {
    #[serde(default = "default_includes")]
    pub include: Vec<String>,
    #[serde(default = "default_excludes")]
    pub exclude: Vec<String>,
}

impl Default for FileSelectionConfig {
    fn default() -> Self {
        Self {
            include: default_includes(),
            exclude: default_excludes(),
        }
    }
}

impl AfmtConfig {
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|error| {
            format!("Failed to read config file: {}: {}", path.display(), error)
        })?;
        let config: AfmtConfig = toml::from_str(&content).map_err(|error| {
            format!("Failed to parse config file: {}: {}", path.display(), error)
        })?;
        config.formatter.validate().map_err(|error| {
            format!(
                "Invalid formatter configuration: {}: {}",
                error,
                path.display()
            )
        })?;
        Ok(config)
    }

    pub fn load(path: Option<&Path>) -> Result<Self, String> {
        path.map_or_else(|| Ok(Self::default()), Self::from_file)
    }

    pub fn base_dir(config_path: Option<&Path>) -> Result<PathBuf, String> {
        match config_path {
            Some(path) => path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
                .map_or_else(
                    || std::env::current_dir().map_err(|error| error.to_string()),
                    |parent| {
                        let base = if parent.is_absolute() {
                            parent.to_path_buf()
                        } else {
                            std::env::current_dir()
                                .map_err(|error| error.to_string())?
                                .join(parent)
                        };
                        std::fs::canonicalize(&base).map_err(|error| {
                            format!(
                                "Failed to resolve config directory {}: {}",
                                base.display(),
                                error
                            )
                        })
                    },
                ),
            None => std::env::current_dir().map_err(|error| error.to_string()),
        }
    }
}

fn default_includes() -> Vec<String> {
    DEFAULT_INCLUDES
        .iter()
        .map(|pattern| (*pattern).to_string())
        .collect()
}

fn default_excludes() -> Vec<String> {
    DEFAULT_EXCLUDES
        .iter()
        .map(|pattern| (*pattern).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_two_key_config_uses_default_file_selection() {
        let config: AfmtConfig = toml::from_str("max_width = 100\nindent_size = 4\n").unwrap();

        assert_eq!(config.formatter.max_width, 100);
        assert_eq!(config.formatter.indent_size, 4);
        assert_eq!(config.files, FileSelectionConfig::default());
    }

    #[test]
    fn custom_file_selection_replaces_each_default_array() {
        let config: AfmtConfig =
            toml::from_str("[files]\ninclude = [\"**/*.foo\"]\nexclude = []\n").unwrap();

        assert_eq!(config.files.include, vec!["**/*.foo"]);
        assert!(config.files.exclude.is_empty());
    }

    #[test]
    fn omitted_file_selection_arrays_default_independently() {
        let config: AfmtConfig = toml::from_str("[files]\nexclude = []\n").unwrap();

        assert_eq!(config.files.include, FileSelectionConfig::default().include);
        assert!(config.files.exclude.is_empty());
    }

    #[test]
    fn invalid_formatter_configuration_is_rejected_before_loading_targets() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("sf-afmt-invalid-config-{unique}"));
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join(".afmt.toml");
        std::fs::write(&path, "indent_size = 0\n").unwrap();

        let error = AfmtConfig::from_file(&path).unwrap_err();

        assert!(error.contains("Invalid formatter configuration"));
        assert!(error.contains("indent_size must be at least 1"));
        assert!(error.contains(&path.display().to_string()));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
