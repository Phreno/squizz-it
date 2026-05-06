use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flashcard {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deck {
    pub name: String,
    pub cards: Vec<Flashcard>,
}

#[derive(Debug, Error)]
pub enum DeckError {
    #[error("could not read decks directory `{path}`: {source}")]
    ReadDeckDirectory {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not open deck file `{path}`: {source}")]
    OpenDeckFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not read CSV deck `{path}`: {source}")]
    ReadDeckCsv {
        path: String,
        #[source]
        source: csv::Error,
    },
    #[error(
        "deck `{deck_name}` is missing required CSV columns (`key` or `question`, and `value` or `réponse`)"
    )]
    MissingRequiredColumns { deck_name: String },
    #[error(
        "deck `{deck_name}` has an invalid row at line {line}: key and value must be non-empty"
    )]
    InvalidCardRow { deck_name: String, line: usize },
    #[error("deck `{deck_name}` has no cards")]
    EmptyDeck { deck_name: String },
}

pub fn discover_decks(decks_dir: &Path) -> Result<Vec<PathBuf>, DeckError> {
    let entries = fs::read_dir(decks_dir).map_err(|source| DeckError::ReadDeckDirectory {
        path: decks_dir.display().to_string(),
        source,
    })?;

    let mut decks = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("csv"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    decks.sort();
    Ok(decks)
}

pub fn filter_decks<'a>(decks: &'a [PathBuf], query: &str) -> Vec<&'a PathBuf> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return decks.iter().collect();
    }
    decks
        .iter()
        .filter(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|name| name.to_lowercase().contains(&needle))
                .unwrap_or(false)
        })
        .collect()
}

pub fn load_deck(path: &Path, delimiter: u8) -> Result<Deck, DeckError> {
    let file = File::open(path).map_err(|source| DeckError::OpenDeckFile {
        path: path.display().to_string(),
        source,
    })?;
    let deck_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("deck")
        .to_string();
    parse_deck_reader(file, &deck_name, delimiter)
}

pub(crate) fn parse_deck_reader<R: Read>(
    reader: R,
    deck_name: &str,
    delimiter: u8,
) -> Result<Deck, DeckError> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .trim(csv::Trim::All)
        .from_reader(reader);

    let headers = csv_reader
        .headers()
        .map_err(|source| DeckError::ReadDeckCsv {
            path: deck_name.to_string(),
            source,
        })?
        .clone();
    let normalized_headers = headers.iter().map(normalize_header).collect::<Vec<_>>();

    let key_index = normalized_headers
        .iter()
        .position(|header| matches!(header.as_str(), "key" | "question" | "key_question"));
    let value_index = normalized_headers.iter().position(|header| {
        matches!(
            header.as_str(),
            "value" | "reponse" | "response" | "answer" | "value_reponse"
        )
    });

    let (Some(key_index), Some(value_index)) = (key_index, value_index) else {
        return Err(DeckError::MissingRequiredColumns {
            deck_name: deck_name.to_string(),
        });
    };

    let mut cards = Vec::new();
    for (zero_based_line, record_result) in csv_reader.records().enumerate() {
        let line = zero_based_line + 2;
        let record = record_result.map_err(|source| DeckError::ReadDeckCsv {
            path: deck_name.to_string(),
            source,
        })?;

        let key = record.get(key_index).unwrap_or("").trim().to_string();
        let value = record.get(value_index).unwrap_or("").trim().to_string();
        if key.is_empty() || value.is_empty() {
            return Err(DeckError::InvalidCardRow {
                deck_name: deck_name.to_string(),
                line,
            });
        }
        cards.push(Flashcard { key, value });
    }

    if cards.is_empty() {
        return Err(DeckError::EmptyDeck {
            deck_name: deck_name.to_string(),
        });
    }

    Ok(Deck {
        name: deck_name.to_string(),
        cards,
    })
}

fn normalize_header(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .replace('é', "e")
        .replace('è', "e")
        .replace('ê', "e")
        .replace('/', "_")
        .replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{DeckError, parse_deck_reader};

    #[test]
    fn parse_valid_csv_with_key_value_headers() {
        let csv_data = "key,value\ncapital france,paris\n2+2,4\n";
        let deck = parse_deck_reader(Cursor::new(csv_data), "sample", b',').expect("valid deck");
        assert_eq!(deck.cards.len(), 2);
        assert_eq!(deck.cards[0].key, "capital france");
        assert_eq!(deck.cards[0].value, "paris");
    }

    #[test]
    fn parse_valid_csv_with_question_reponse_headers() {
        let csv_data = "question,réponse\nhello,bonjour\n";
        let deck = parse_deck_reader(Cursor::new(csv_data), "sample", b',').expect("valid deck");
        assert_eq!(deck.cards.len(), 1);
        assert_eq!(deck.cards[0].key, "hello");
        assert_eq!(deck.cards[0].value, "bonjour");
    }

    #[test]
    fn fail_when_required_headers_are_missing() {
        let csv_data = "front,back\nquestion,answer\n";
        let error = parse_deck_reader(Cursor::new(csv_data), "sample", b',').expect_err("invalid");
        assert!(matches!(error, DeckError::MissingRequiredColumns { .. }));
    }
}
