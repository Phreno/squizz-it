/// Compute the Levenshtein edit distance between two strings.
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();

    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0; b_len + 1];

    for i in 1..=a_len {
        curr[0] = i;
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1)
                .min(curr[j - 1] + 1)
                .min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len]
}

/// Check whether `input` is a near-match for `expected`.
///
/// Threshold: at most max(1, (expected_len + 2) / 3) edits.
/// Both strings are compared case-insensitively.
pub fn is_near_match(expected: &str, input: &str) -> bool {
    let e = expected.to_lowercase();
    let i = input.to_lowercase();
    if e == i {
        return false; // exact match, not a "near" match
    }
    let len = e.chars().count();
    let threshold = 1.max((len + 2) / 3);
    levenshtein_distance(&e, &i) <= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_identical_strings() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn distance_empty_strings() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", ""), 3);
        assert_eq!(levenshtein_distance("", "xyz"), 3);
    }

    #[test]
    fn distance_single_edit() {
        assert_eq!(levenshtein_distance("kitten", "sitten"), 1);
        assert_eq!(levenshtein_distance("paris", "prais"), 2);
    }

    #[test]
    fn distance_longer_strings() {
        assert_eq!(levenshtein_distance("saturday", "sunday"), 3);
    }

    #[test]
    fn near_match_typo() {
        assert!(is_near_match("Paris", "Prais"));
    }

    #[test]
    fn near_match_one_char_off() {
        assert!(is_near_match("hello", "helo"));
    }

    #[test]
    fn near_match_exact_is_false() {
        assert!(!is_near_match("Paris", "Paris"));
        assert!(!is_near_match("Paris", "paris"));
    }

    #[test]
    fn near_match_too_far() {
        assert!(!is_near_match("Paris", "London"));
    }

    #[test]
    fn near_match_short_word_threshold_is_one() {
        // "ab" → threshold = max(1, (2+2)/3) = 1
        assert!(is_near_match("ab", "a"));
        assert!(!is_near_match("ab", "xyz"));
    }
}
