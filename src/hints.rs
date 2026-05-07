use crate::config::HintMode;

/// Generate a hint string based on the expected answer, consecutive error count, and hint mode.
pub fn generate_hint(expected: &str, consecutive_errors: u32, mode: HintMode) -> String {
    match mode {
        HintMode::None => String::new(),
        HintMode::Immediate => expected.to_string(),
        HintMode::Progressive => progressive_hint(expected, consecutive_errors),
    }
}

fn progressive_hint(expected: &str, consecutive_errors: u32) -> String {
    match consecutive_errors {
        0 => String::new(),
        1 => {
            // Show character count as underscores
            let blanks: String = expected
                .chars()
                .map(|ch| if ch == ' ' { ' ' } else { '_' })
                .collect();
            format!("Indice : {blanks} ({} caractères)", expected.chars().count())
        }
        2 => {
            // Reveal first letter, rest as underscores
            let mut chars = expected.chars();
            let first = chars.next().unwrap_or('_');
            let rest: String = chars.map(|ch| if ch == ' ' { ' ' } else { '_' }).collect();
            format!("Indice : {first}{rest}")
        }
        _ => {
            // Full answer
            expected.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::HintMode;

    #[test]
    fn progressive_first_error_shows_blanks() {
        let hint = generate_hint("Paris", 1, HintMode::Progressive);
        assert_eq!(hint, "Indice : _____ (5 caractères)");
    }

    #[test]
    fn progressive_first_error_preserves_spaces() {
        let hint = generate_hint("New York", 1, HintMode::Progressive);
        assert_eq!(hint, "Indice : ___ ____ (8 caractères)");
    }

    #[test]
    fn progressive_second_error_reveals_first_letter() {
        let hint = generate_hint("Paris", 2, HintMode::Progressive);
        assert_eq!(hint, "Indice : P____");
    }

    #[test]
    fn progressive_third_error_reveals_full_answer() {
        let hint = generate_hint("Paris", 3, HintMode::Progressive);
        assert_eq!(hint, "Paris");
    }

    #[test]
    fn immediate_mode_always_shows_answer() {
        assert_eq!(generate_hint("Paris", 1, HintMode::Immediate), "Paris");
        assert_eq!(generate_hint("Paris", 2, HintMode::Immediate), "Paris");
    }

    #[test]
    fn none_mode_returns_empty() {
        assert_eq!(generate_hint("Paris", 1, HintMode::None), "");
        assert_eq!(generate_hint("Paris", 3, HintMode::None), "");
    }

    #[test]
    fn progressive_zero_errors_returns_empty() {
        assert_eq!(generate_hint("Paris", 0, HintMode::Progressive), "");
    }
}
