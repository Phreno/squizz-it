use rand::{SeedableRng, rngs::StdRng, seq::SliceRandom};
use thiserror::Error;

use crate::{
    config::{AnswerMode, GameConfig},
    deck::Flashcard,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionConfig {
    pub answer_mode: AnswerMode,
    pub normalize_whitespace: bool,
    pub shuffle_seed: Option<u64>,
    pub pre_ordered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmitOutcome {
    Incorrect,
    AdvanceCard,
    AdvanceStage,
    RoundRestarted,
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("session cannot start with an empty deck")]
    EmptyDeck,
}

#[derive(Debug, Clone)]
pub struct Session {
    cards: Vec<Flashcard>,
    stage_len: usize,
    cursor: usize,
    round: usize,
    config: SessionConfig,
    rng: StdRng,
}

impl From<GameConfig> for SessionConfig {
    fn from(value: GameConfig) -> Self {
        Self {
            answer_mode: value.answer_mode,
            normalize_whitespace: value.normalize_whitespace,
            shuffle_seed: value.shuffle_seed,
            pre_ordered: value.srs_ordering,
        }
    }
}

impl Session {
    pub fn new(mut cards: Vec<Flashcard>, config: SessionConfig) -> Result<Self, SessionError> {
        if cards.is_empty() {
            return Err(SessionError::EmptyDeck);
        }
        let mut rng = config
            .shuffle_seed
            .map(StdRng::seed_from_u64)
            .unwrap_or_else(StdRng::from_entropy);
        if !config.pre_ordered {
            cards.shuffle(&mut rng);
        }

        Ok(Self {
            cards,
            stage_len: 1,
            cursor: 0,
            round: 1,
            config,
            rng,
        })
    }

    pub fn current_card(&self) -> &Flashcard {
        &self.cards[self.cursor]
    }

    pub fn progress(&self) -> (usize, usize, usize) {
        (self.stage_len, self.cards.len(), self.cursor + 1)
    }
    pub fn round(&self) -> usize {
        self.round
    }

    pub fn is_replay_card(&self) -> bool {
        self.cursor + 1 < self.stage_len
    }

    pub fn submit_answer(&mut self, answer: &str) -> SubmitOutcome {
        let expected = &self.current_card().value;
        if !answers_match(expected, answer, self.config) {
            return SubmitOutcome::Incorrect;
        }

        if self.cursor + 1 < self.stage_len {
            self.cursor += 1;
            return SubmitOutcome::AdvanceCard;
        }

        if self.stage_len == self.cards.len() {
            self.cards.shuffle(&mut self.rng);
            self.stage_len = 1;
            self.cursor = 0;
            self.round += 1;
            return SubmitOutcome::RoundRestarted;
        }

        self.stage_len += 1;
        self.cursor = 0;
        SubmitOutcome::AdvanceStage
    }

    #[cfg(test)]
    pub fn ordered_keys_for_tests(&self) -> Vec<String> {
        self.cards.iter().map(|card| card.key.clone()).collect()
    }
}

fn answers_match(expected: &str, input: &str, config: SessionConfig) -> bool {
    let expected_norm = normalize(expected, config);
    let input_norm = normalize(input, config);
    expected_norm == input_norm
}

fn normalize(raw: &str, config: SessionConfig) -> String {
    let trimmed = if config.normalize_whitespace {
        raw.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        raw.to_string()
    };

    match config.answer_mode {
        AnswerMode::Exact => trimmed.trim().to_string(),
        AnswerMode::CaseInsensitive => trimmed.trim().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{config::AnswerMode, deck::Flashcard};

    use super::{Session, SessionConfig, SubmitOutcome};

    fn test_cards() -> Vec<Flashcard> {
        vec![
            Flashcard {
                key: "A".to_string(),
                value: "Alpha".to_string(),
            },
            Flashcard {
                key: "B".to_string(),
                value: "Beta".to_string(),
            },
        ]
    }

    fn config_with_seed(seed: u64) -> SessionConfig {
        SessionConfig {
            answer_mode: AnswerMode::CaseInsensitive,
            normalize_whitespace: true,
            shuffle_seed: Some(seed),
            pre_ordered: false,
        }
    }

    #[test]
    fn wrong_answer_keeps_same_card() {
        let mut session = Session::new(test_cards(), config_with_seed(1)).expect("session");
        let first_key = session.current_card().key.clone();
        let outcome = session.submit_answer("wrong");
        assert_eq!(outcome, SubmitOutcome::Incorrect);
        assert_eq!(session.current_card().key, first_key);
    }

    #[test]
    fn correct_answer_advances_stage_then_requires_full_sequence() {
        let mut session = Session::new(test_cards(), config_with_seed(2)).expect("session");
        let first_answer = session.current_card().value.clone();
        let stage_one = session.submit_answer(&first_answer);
        assert_eq!(stage_one, SubmitOutcome::AdvanceStage);
        let (stage_len, total, cursor) = session.progress();
        assert_eq!((stage_len, total, cursor), (2, 2, 1));
    }

    #[test]
    fn session_restarts_round_after_finishing_last_stage() {
        let mut session = Session::new(test_cards(), config_with_seed(2)).expect("session");
        let a1 = session.current_card().value.clone();
        assert_eq!(session.submit_answer(&a1), SubmitOutcome::AdvanceStage);
        let a2 = session.current_card().value.clone();
        assert_eq!(session.submit_answer(&a2), SubmitOutcome::AdvanceCard);
        let a3 = session.current_card().value.clone();
        assert_eq!(session.submit_answer(&a3), SubmitOutcome::RoundRestarted);
        let (stage_len, _, cursor) = session.progress();
        assert_eq!((stage_len, cursor), (1, 1));
        assert_eq!(session.round(), 2);
    }

    #[test]
    fn deterministic_shuffle_with_seed() {
        let s1 = Session::new(test_cards(), config_with_seed(42)).expect("session");
        let s2 = Session::new(test_cards(), config_with_seed(42)).expect("session");
        assert_eq!(s1.ordered_keys_for_tests(), s2.ordered_keys_for_tests());
    }

    #[test]
    fn answer_matching_can_be_exact() {
        let mut session = Session::new(
            test_cards(),
            SessionConfig {
                answer_mode: AnswerMode::Exact,
                normalize_whitespace: true,
                shuffle_seed: Some(3),
                pre_ordered: false,
            },
        )
        .expect("session");
        let expected = session.current_card().value.clone();
        let outcome = session.submit_answer(&expected.to_lowercase());
        assert_eq!(outcome, SubmitOutcome::Incorrect);
    }

    #[test]
    fn replay_cards_are_detected() {
        let mut session = Session::new(test_cards(), config_with_seed(2)).expect("session");
        let a1 = session.current_card().value.clone();
        assert_eq!(session.submit_answer(&a1), SubmitOutcome::AdvanceStage);
        assert!(session.is_replay_card());
        let a2 = session.current_card().value.clone();
        assert_eq!(session.submit_answer(&a2), SubmitOutcome::AdvanceCard);
        assert!(!session.is_replay_card());
    }
}
