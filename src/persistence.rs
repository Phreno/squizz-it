use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("could not create data directory `{path}`: {source}")]
    CreateDir {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not read store file `{path}`: {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse store file `{path}`: {source}")]
    ParseFile {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("could not write store file `{path}`: {source}")]
    WriteFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not serialize store: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardStats {
    pub correct_count: u32,
    pub incorrect_count: u32,
    pub streak: u32,
    pub best_streak: u32,
    pub last_seen: u64,
    #[serde(default = "default_easiness")]
    pub easiness_factor: f64,
    #[serde(default)]
    pub interval_days: f64,
    #[serde(default)]
    pub next_review: u64,
}

impl Default for CardStats {
    fn default() -> Self {
        Self {
            correct_count: 0,
            incorrect_count: 0,
            streak: 0,
            best_streak: 0,
            last_seen: 0,
            easiness_factor: 2.5,
            interval_days: 0.0,
            next_review: 0,
        }
    }
}

fn default_easiness() -> f64 {
    2.5
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeckProgress {
    pub total_sessions: u32,
    pub best_round: u32,
    pub last_played: u64,
    pub cards: HashMap<String, CardStats>,
}

const SECONDS_PER_DAY: u64 = 86_400;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalStats {
    pub last_session_day: u64,
    pub daily_streak: u32,
}

impl GlobalStats {
    pub fn record_session(&mut self, now: u64) {
        let today = now / SECONDS_PER_DAY;
        if today == self.last_session_day {
            return; // already recorded today
        }
        if today == self.last_session_day + 1 {
            self.daily_streak += 1;
        } else {
            self.daily_streak = 1;
        }
        self.last_session_day = today;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Store {
    pub decks: HashMap<String, DeckProgress>,
    #[serde(default)]
    pub global_stats: GlobalStats,
}

impl CardStats {
    pub fn record_correct(&mut self) {
        self.correct_count += 1;
        self.streak += 1;
        if self.streak > self.best_streak {
            self.best_streak = self.streak;
        }
        self.last_seen = now_unix();
    }

    pub fn record_incorrect(&mut self) {
        self.incorrect_count += 1;
        self.streak = 0;
        self.last_seen = now_unix();
    }

    pub fn accuracy(&self) -> f64 {
        let total = self.correct_count + self.incorrect_count;
        if total == 0 {
            return 0.0;
        }
        self.correct_count as f64 / total as f64
    }

    pub fn total_reviews(&self) -> u32 {
        self.correct_count + self.incorrect_count
    }
}

impl DeckProgress {
    pub fn record_session_start(&mut self) {
        self.total_sessions += 1;
        self.last_played = now_unix();
    }

    pub fn update_best_round(&mut self, round: u32) {
        if round > self.best_round {
            self.best_round = round;
        }
    }

    pub fn card_stats_mut(&mut self, card_key: &str) -> &mut CardStats {
        self.cards.entry(card_key.to_string()).or_default()
    }

    pub fn overall_accuracy(&self) -> f64 {
        let (correct, total) = self.cards.values().fold((0u32, 0u32), |(c, t), stats| {
            (
                c + stats.correct_count,
                t + stats.correct_count + stats.incorrect_count,
            )
        });
        if total == 0 {
            0.0
        } else {
            correct as f64 / total as f64
        }
    }

    pub fn total_reviews(&self) -> u32 {
        self.cards
            .values()
            .map(|s| s.correct_count + s.incorrect_count)
            .sum()
    }
}

impl Store {
    pub fn deck_progress_mut(&mut self, deck_name: &str) -> &mut DeckProgress {
        self.decks.entry(deck_name.to_string()).or_default()
    }
}

pub fn default_store_path() -> PathBuf {
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".local").join("share")
        });
    data_home.join("squizz-it").join("stats.json")
}

pub fn load_store(path: &Path) -> Result<Store, StoreError> {
    if !path.exists() {
        return Ok(Store::default());
    }
    let content = fs::read_to_string(path).map_err(|source| StoreError::ReadFile {
        path: path.display().to_string(),
        source,
    })?;
    serde_json::from_str(&content).map_err(|source| StoreError::ParseFile {
        path: path.display().to_string(),
        source,
    })
}

pub fn save_store(store: &Store, path: &Path) -> Result<(), StoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| StoreError::CreateDir {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let content = serde_json::to_string_pretty(store)?;
    fs::write(path, content).map_err(|source| StoreError::WriteFile {
        path: path.display().to_string(),
        source,
    })
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn card_stats_record_correct_updates_streak() {
        let mut stats = CardStats::default();
        stats.record_correct();
        assert_eq!(stats.correct_count, 1);
        assert_eq!(stats.streak, 1);
        assert_eq!(stats.best_streak, 1);
    }

    #[test]
    fn card_stats_record_incorrect_resets_streak() {
        let mut stats = CardStats::default();
        stats.record_correct();
        stats.record_correct();
        stats.record_incorrect();
        assert_eq!(stats.streak, 0);
        assert_eq!(stats.best_streak, 2);
        assert_eq!(stats.incorrect_count, 1);
    }

    #[test]
    fn card_stats_accuracy_calculation() {
        let mut stats = CardStats::default();
        stats.record_correct();
        stats.record_correct();
        stats.record_incorrect();
        let expected = 2.0 / 3.0;
        assert!((stats.accuracy() - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn card_stats_accuracy_empty() {
        let stats = CardStats::default();
        assert_eq!(stats.accuracy(), 0.0);
    }

    #[test]
    fn deck_progress_overall_accuracy() {
        let mut progress = DeckProgress::default();
        progress.card_stats_mut("a").record_correct();
        progress.card_stats_mut("a").record_correct();
        progress.card_stats_mut("b").record_incorrect();
        let expected = 2.0 / 3.0;
        assert!((progress.overall_accuracy() - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn deck_progress_update_best_round_keeps_maximum() {
        let mut progress = DeckProgress::default();
        progress.update_best_round(3);
        assert_eq!(progress.best_round, 3);
        progress.update_best_round(2);
        assert_eq!(progress.best_round, 3);
        progress.update_best_round(5);
        assert_eq!(progress.best_round, 5);
    }

    #[test]
    fn store_round_trip_with_tempfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test_stats.json");

        let mut store = Store::default();
        let progress = store.deck_progress_mut("test_deck");
        progress.record_session_start();
        progress.card_stats_mut("card1").record_correct();
        progress.card_stats_mut("card1").record_correct();
        progress.card_stats_mut("card2").record_incorrect();

        save_store(&store, &path).unwrap();
        let loaded = load_store(&path).unwrap();

        let deck = loaded.decks.get("test_deck").unwrap();
        assert_eq!(deck.total_sessions, 1);

        let card1 = deck.cards.get("card1").unwrap();
        assert_eq!(card1.correct_count, 2);
        assert_eq!(card1.streak, 2);

        let card2 = deck.cards.get("card2").unwrap();
        assert_eq!(card2.incorrect_count, 1);
        assert_eq!(card2.streak, 0);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let store =
            load_store(Path::new("/tmp/__squizz_it_missing_test__.json")).unwrap();
        assert!(store.decks.is_empty());
    }

    #[test]
    fn global_stats_consecutive_days_increment_streak() {
        let mut stats = GlobalStats::default();
        stats.record_session(SECONDS_PER_DAY * 10); // day 10
        assert_eq!(stats.daily_streak, 1);
        stats.record_session(SECONDS_PER_DAY * 11); // day 11
        assert_eq!(stats.daily_streak, 2);
        stats.record_session(SECONDS_PER_DAY * 12); // day 12
        assert_eq!(stats.daily_streak, 3);
    }

    #[test]
    fn global_stats_gap_resets_streak() {
        let mut stats = GlobalStats::default();
        stats.record_session(SECONDS_PER_DAY * 10);
        stats.record_session(SECONDS_PER_DAY * 11);
        assert_eq!(stats.daily_streak, 2);
        stats.record_session(SECONDS_PER_DAY * 15); // 3-day gap
        assert_eq!(stats.daily_streak, 1);
    }

    #[test]
    fn global_stats_same_day_does_not_increment() {
        let mut stats = GlobalStats::default();
        stats.record_session(SECONDS_PER_DAY * 10);
        stats.record_session(SECONDS_PER_DAY * 10 + 3600);
        assert_eq!(stats.daily_streak, 1);
    }

    #[test]
    fn deck_progress_total_reviews() {
        let mut progress = DeckProgress::default();
        progress.card_stats_mut("a").record_correct();
        progress.card_stats_mut("a").record_correct();
        progress.card_stats_mut("b").record_incorrect();
        progress.card_stats_mut("b").record_correct();
        assert_eq!(progress.total_reviews(), 4);
    }
}
