use crate::{
    deck::Flashcard,
    persistence::{CardStats, DeckProgress},
};

const MIN_EF: f64 = 1.3;
const SECONDS_PER_DAY: f64 = 86_400.0;

/// Compute how urgently a card needs review.
/// Higher values indicate greater urgency. New cards receive maximum priority.
pub fn priority_score(stats: &CardStats, now: u64) -> f64 {
    if stats.total_reviews() == 0 {
        return f64::MAX;
    }
    if stats.next_review == 0 {
        return f64::MAX - 1.0;
    }
    let overdue_secs = now as f64 - stats.next_review as f64;
    let interval_secs = stats.interval_days * SECONDS_PER_DAY;
    if interval_secs <= 0.0 {
        return overdue_secs;
    }
    overdue_secs / interval_secs
}

/// Update the SRS schedule for a card after a frontier review.
///
/// Uses SM-2 with fixed quality grades: correct → 4, incorrect → 1.
/// Quality 4 keeps the easiness factor stable; quality 1 drops it sharply.
/// Interval progression on correct answers: 1 day → 6 days → interval × EF.
/// Incorrect answers reset the interval to 1 day.
pub fn update_schedule(stats: &mut CardStats, correct: bool, now: u64) {
    let quality: f64 = if correct { 4.0 } else { 1.0 };

    let new_ef =
        stats.easiness_factor + (0.1 - (5.0 - quality) * (0.08 + (5.0 - quality) * 0.02));
    stats.easiness_factor = new_ef.max(MIN_EF);

    if correct {
        if stats.interval_days < 1.0 {
            stats.interval_days = 1.0;
        } else if stats.interval_days < 6.0 {
            stats.interval_days = 6.0;
        } else {
            stats.interval_days *= stats.easiness_factor;
        }
    } else {
        stats.interval_days = 1.0;
    }

    stats.next_review = now + (stats.interval_days * SECONDS_PER_DAY) as u64;
}

/// Sort cards by SRS priority (most urgent first).
/// Cards without stats (new) appear before reviewed cards.
/// The sort is stable, so cards with equal priority keep their original order.
pub fn order_by_priority(
    mut cards: Vec<Flashcard>,
    progress: &DeckProgress,
    now: u64,
) -> Vec<Flashcard> {
    cards.sort_by(|a, b| {
        let pa = card_priority(&a.key, progress, now);
        let pb = card_priority(&b.key, progress, now);
        pb.partial_cmp(&pa).unwrap_or(std::cmp::Ordering::Equal)
    });
    cards
}

fn card_priority(key: &str, progress: &DeckProgress, now: u64) -> f64 {
    match progress.cards.get(key) {
        Some(stats) => priority_score(stats, now),
        None => f64::MAX,
    }
}

#[cfg(test)]
mod tests {
    use crate::persistence::DeckProgress;

    use super::*;

    fn make_stats(ef: f64, interval: f64, next_review: u64, reviews: u32) -> CardStats {
        CardStats {
            correct_count: reviews,
            interval_days: interval,
            next_review,
            easiness_factor: ef,
            last_seen: 1000,
            streak: reviews,
            best_streak: reviews,
            ..CardStats::default()
        }
    }

    #[test]
    fn new_card_has_maximum_priority() {
        let stats = CardStats::default();
        assert_eq!(priority_score(&stats, 1000), f64::MAX);
    }

    #[test]
    fn overdue_card_has_positive_priority() {
        let stats = make_stats(2.5, 1.0, 500, 3);
        let score = priority_score(&stats, 1000);
        assert!(score > 0.0);
    }

    #[test]
    fn card_not_yet_due_has_negative_priority() {
        let stats = make_stats(2.5, 10.0, 2_000_000, 3);
        let score = priority_score(&stats, 1000);
        assert!(score < 0.0);
    }

    #[test]
    fn correct_answer_increases_interval() {
        let mut stats = CardStats {
            correct_count: 1,
            ..CardStats::default()
        };

        update_schedule(&mut stats, true, 1000);
        assert_eq!(stats.interval_days, 1.0);

        update_schedule(&mut stats, true, 1000);
        assert_eq!(stats.interval_days, 6.0);

        let ef = stats.easiness_factor;
        update_schedule(&mut stats, true, 1000);
        assert!((stats.interval_days - 6.0 * ef).abs() < 0.001);
    }

    #[test]
    fn incorrect_answer_resets_interval() {
        let mut stats = make_stats(2.5, 30.0, 500, 10);
        update_schedule(&mut stats, false, 1000);
        assert_eq!(stats.interval_days, 1.0);
    }

    #[test]
    fn easiness_factor_never_below_minimum() {
        let mut stats = CardStats {
            correct_count: 1,
            ..CardStats::default()
        };
        for _ in 0..20 {
            update_schedule(&mut stats, false, 1000);
        }
        assert!(stats.easiness_factor >= MIN_EF);
    }

    #[test]
    fn next_review_is_set_after_update() {
        let mut stats = CardStats {
            correct_count: 1,
            ..CardStats::default()
        };
        update_schedule(&mut stats, true, 10_000);
        assert_eq!(stats.next_review, 10_000 + SECONDS_PER_DAY as u64);
    }

    #[test]
    fn order_puts_new_cards_before_not_due() {
        use crate::deck::Flashcard;

        let cards = vec![
            Flashcard {
                key: "known".into(),
                value: "v1".into(),
            },
            Flashcard {
                key: "new".into(),
                value: "v2".into(),
            },
        ];

        let mut progress = DeckProgress::default();
        let stats = progress.card_stats_mut("known");
        stats.correct_count = 5;
        stats.interval_days = 30.0;
        stats.next_review = 2_000_000;

        let ordered = order_by_priority(cards, &progress, 1000);
        assert_eq!(ordered[0].key, "new");
        assert_eq!(ordered[1].key, "known");
    }

    #[test]
    fn order_puts_overdue_before_not_due() {
        use crate::deck::Flashcard;

        let cards = vec![
            Flashcard {
                key: "fresh".into(),
                value: "v1".into(),
            },
            Flashcard {
                key: "overdue".into(),
                value: "v2".into(),
            },
        ];

        let mut progress = DeckProgress::default();

        let fresh = progress.card_stats_mut("fresh");
        fresh.correct_count = 3;
        fresh.interval_days = 10.0;
        fresh.next_review = 2_000_000; // far in the future

        let overdue = progress.card_stats_mut("overdue");
        overdue.correct_count = 3;
        overdue.interval_days = 1.0;
        overdue.next_review = 500; // past

        let ordered = order_by_priority(cards, &progress, 1000);
        assert_eq!(ordered[0].key, "overdue");
        assert_eq!(ordered[1].key, "fresh");
    }
}
