use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AnswerMode {
    Exact,
    #[default]
    CaseInsensitive,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlayMode {
    #[default]
    Simon,
    Classic,
    Reverse,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CsvConfig {
    #[serde(default = "default_delimiter")]
    pub delimiter: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GameConfig {
    #[serde(default)]
    pub answer_mode: AnswerMode,
    #[serde(default = "default_true")]
    pub normalize_whitespace: bool,
    #[serde(default)]
    pub shuffle_seed: Option<u64>,
    #[serde(default = "default_true")]
    pub srs_ordering: bool,
    #[serde(default)]
    pub play_mode: PlayMode,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    #[serde(default = "default_decks_dir")]
    pub decks_dir: PathBuf,
    #[serde(default)]
    pub csv: CsvConfig,
    #[serde(default)]
    pub game: GameConfig,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read config file `{path}`: {source}")]
    ReadConfig {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse config file `{path}`: {source}")]
    ParseConfig {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid CSV delimiter `{delimiter}` in config: use exactly one UTF-8 character")]
    InvalidDelimiter { delimiter: String },
}

impl Default for CsvConfig {
    fn default() -> Self {
        Self {
            delimiter: default_delimiter(),
        }
    }
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            answer_mode: AnswerMode::default(),
            normalize_whitespace: default_true(),
            shuffle_seed: None,
            srs_ordering: default_true(),
            play_mode: PlayMode::default(),
        }
    }
}

impl std::fmt::Display for PlayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayMode::Simon => write!(f, "simon"),
            PlayMode::Classic => write!(f, "classic"),
            PlayMode::Reverse => write!(f, "reverse"),
        }
    }
}

impl std::str::FromStr for PlayMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "simon" => Ok(PlayMode::Simon),
            "classic" => Ok(PlayMode::Classic),
            "reverse" => Ok(PlayMode::Reverse),
            other => Err(format!("unknown play mode `{other}`; use simon, classic, or reverse")),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            decks_dir: default_decks_dir(),
            csv: CsvConfig::default(),
            game: GameConfig::default(),
        }
    }
}

impl CsvConfig {
    pub fn delimiter_byte(&self) -> Result<u8, ConfigError> {
        let mut chars = self.delimiter.chars();
        let Some(ch) = chars.next() else {
            return Err(ConfigError::InvalidDelimiter {
                delimiter: self.delimiter.clone(),
            });
        };
        if chars.next().is_some() || !ch.is_ascii() {
            return Err(ConfigError::InvalidDelimiter {
                delimiter: self.delimiter.clone(),
            });
        }
        Ok(ch as u8)
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|source| ConfigError::ReadConfig {
            path: path.display().to_string(),
            source,
        })?;
        let config =
            toml::from_str::<Self>(&content).map_err(|source| ConfigError::ParseConfig {
                path: path.display().to_string(),
                source,
            })?;
        config.validate()?;
        Ok(config)
    }

    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            Self::load(path)
        } else {
            Ok(Self::default())
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        self.csv.delimiter_byte()?;
        Ok(())
    }
}

fn default_decks_dir() -> PathBuf {
    PathBuf::from("decks")
}

fn default_delimiter() -> String {
    ",".to_string()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{AnswerMode, AppConfig, CsvConfig, GameConfig, PlayMode};

    #[test]
    fn parse_config_toml() {
        let raw = r#"
decks_dir = "fixtures/decks"

[csv]
delimiter = ";"

[game]
answer_mode = "exact"
normalize_whitespace = false
shuffle_seed = 42
"#;

        let parsed: AppConfig = toml::from_str(raw).expect("valid toml");
        assert_eq!(parsed.decks_dir, PathBuf::from("fixtures/decks"));
        assert_eq!(
            parsed.csv,
            CsvConfig {
                delimiter: ";".to_string()
            }
        );
        assert_eq!(
            parsed.game,
            GameConfig {
                answer_mode: AnswerMode::Exact,
                normalize_whitespace: false,
                shuffle_seed: Some(42),
                srs_ordering: true,
                play_mode: PlayMode::Simon,
            }
        );
    }

    #[test]
    fn fallback_to_default_when_file_is_missing() {
        let path = PathBuf::from("__definitely_missing_squizz_it_config.toml");
        let config = AppConfig::load_or_default(&path).expect("default config");
        assert_eq!(config.decks_dir, PathBuf::from("decks"));
    }
}
