use std::{
    io,
    path::PathBuf,
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use thiserror::Error;

use crate::{
    config::AppConfig,
    deck::{DeckError, discover_decks, filter_decks, load_deck},
    game::{Session, SessionConfig, SessionError, SubmitOutcome},
};

#[derive(Debug, Clone, Default)]
pub struct UiOptions {
    pub deck: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Error)]
pub enum UiError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Config(#[from] crate::config::ConfigError),
    #[error(transparent)]
    Deck(#[from] DeckError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("no deck found in `{decks_dir}`")]
    NoDeckFound { decks_dir: String },
    #[error("no deck matches `{needle}`")]
    NoDeckMatches { needle: String },
    #[error("deck selector `{needle}` is ambiguous; use --deck with exact name")]
    AmbiguousDeckMatch { needle: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    DeckSelect,
    Playing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MessageTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
struct StatusMessage {
    text: String,
    tone: MessageTone,
}

impl StatusMessage {
    fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: MessageTone::Info,
        }
    }
    fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: MessageTone::Success,
        }
    }
    fn warning(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: MessageTone::Warning,
        }
    }
    fn error(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            tone: MessageTone::Error,
        }
    }
}

#[derive(Debug, Clone)]
struct App {
    screen: Screen,
    decks: Vec<PathBuf>,
    query: String,
    selected_deck_index: usize,
    answer_input: String,
    status: StatusMessage,
    should_exit: bool,
    delimiter: u8,
    session_config: SessionConfig,
    deck_name: String,
    session: Option<Session>,
    total_cards: usize,
}

impl App {
    fn new(config: &AppConfig, decks: Vec<PathBuf>, delimiter: u8, options: &UiOptions) -> Self {
        Self {
            screen: Screen::DeckSelect,
            decks,
            query: options.search.clone().unwrap_or_default(),
            selected_deck_index: 0,
            answer_input: String::new(),
            status: StatusMessage::info("Choisis un deck, puis valide avec Entrée."),
            should_exit: false,
            delimiter,
            session_config: SessionConfig::from(config.game.clone()),
            deck_name: String::new(),
            session: None,
            total_cards: 0,
        }
    }

    fn filtered_decks(&self) -> Vec<PathBuf> {
        filter_decks(&self.decks, &self.query)
            .into_iter()
            .cloned()
            .collect()
    }

    fn clamp_selection(&mut self, filtered_len: usize) {
        if filtered_len == 0 {
            self.selected_deck_index = 0;
            return;
        }
        if self.selected_deck_index >= filtered_len {
            self.selected_deck_index = filtered_len - 1;
        }
    }

    fn start_session(&mut self, deck_path: PathBuf) -> Result<(), UiError> {
        let deck = load_deck(&deck_path, self.delimiter)?;
        self.deck_name = deck.name.clone();
        self.total_cards = deck.cards.len();
        self.session = Some(Session::new(deck.cards, self.session_config)?);
        self.answer_input.clear();
        self.status = StatusMessage::success("Session lancée. Bonne mémoire.");
        self.screen = Screen::Playing;
        Ok(())
    }
}

pub fn run(config: &AppConfig, options: UiOptions) -> Result<(), UiError> {
    let delimiter = config.csv.delimiter_byte()?;
    let decks = discover_decks(&config.decks_dir)?;
    if decks.is_empty() {
        return Err(UiError::NoDeckFound {
            decks_dir: config.decks_dir.display().to_string(),
        });
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_event_loop(&mut terminal, config, options, decks, delimiter);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: &AppConfig,
    options: UiOptions,
    decks: Vec<PathBuf>,
    delimiter: u8,
) -> Result<(), UiError> {
    let mut app = App::new(config, decks, delimiter, &options);

    if let Some(selector) = options.deck {
        let path = select_deck_from_selector(&app.decks, &selector)?;
        app.start_session(path)?;
    } else {
        let filtered_len = app.filtered_decks().len();
        app.clamp_selection(filtered_len);
    }

    while !app.should_exit {
        terminal.draw(|frame| draw(frame, &app))?;
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                handle_key_event(&mut app, key)?;
            }
        }
    }

    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) -> Result<(), UiError> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        app.should_exit = true;
        return Ok(());
    }
    if matches!(key.code, KeyCode::Char('q')) {
        app.should_exit = true;
        return Ok(());
    }

    match app.screen {
        Screen::DeckSelect => handle_deck_select_key(app, key),
        Screen::Playing => handle_play_key(app, key),
    }
}

fn handle_deck_select_key(app: &mut App, key: KeyEvent) -> Result<(), UiError> {
    match key.code {
        KeyCode::Up => {
            if app.selected_deck_index > 0 {
                app.selected_deck_index -= 1;
            }
        }
        KeyCode::Down => {
            let filtered = app.filtered_decks();
            if app.selected_deck_index + 1 < filtered.len() {
                app.selected_deck_index += 1;
            }
        }
        KeyCode::Enter => {
            let filtered = app.filtered_decks();
            if filtered.is_empty() {
                app.status = StatusMessage::warning("Aucun deck disponible pour ce filtre.");
            } else {
                let selected = filtered[app.selected_deck_index].clone();
                app.start_session(selected)?;
            }
        }
        KeyCode::Esc => {
            app.query.clear();
            app.selected_deck_index = 0;
            app.status = StatusMessage::info("Filtre réinitialisé.");
        }
        KeyCode::Backspace => {
            app.query.pop();
            let filtered_len = app.filtered_decks().len();
            app.clamp_selection(filtered_len);
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.query.push(ch);
            let filtered_len = app.filtered_decks().len();
            app.clamp_selection(filtered_len);
        }
        _ => {}
    }
    Ok(())
}

fn handle_play_key(app: &mut App, key: KeyEvent) -> Result<(), UiError> {
    let Some(session) = app.session.as_mut() else {
        app.status = StatusMessage::error("Session absente.");
        app.screen = Screen::DeckSelect;
        return Ok(());
    };

    match key.code {
        KeyCode::Enter => {
            let answer = app.answer_input.trim_end().to_string();
            app.answer_input.clear();
            match session.submit_answer(&answer) {
                SubmitOutcome::Incorrect => {
                    app.status = StatusMessage::error("Incorrect. Rejoue cette carte.");
                }
                SubmitOutcome::AdvanceCard => {
                    app.status = StatusMessage::success("Correct. Continue la séquence.");
                }
                SubmitOutcome::AdvanceStage => {
                    app.status =
                        StatusMessage::success("Séquence validée. Nouvelle carte ajoutée.");
                }
                SubmitOutcome::RoundRestarted => {
                    app.status =
                        StatusMessage::success("Deck validé. Nouveau mélange, nouvelle manche.");
                }
            }
        }
        KeyCode::Backspace => {
            app.answer_input.pop();
        }
        KeyCode::Esc => {
            app.answer_input.clear();
        }
        KeyCode::Char(ch) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.answer_input.push(ch);
        }
        _ => {}
    }

    Ok(())
}

fn draw(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::DeckSelect => draw_deck_screen(frame, app),
        Screen::Playing => draw_play_screen(frame, app),
    }
}

fn draw_deck_screen(frame: &mut Frame, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "Squizz-it",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" · Deck Browser"),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Accueil"));
    frame.render_widget(title, areas[0]);

    let query = Paragraph::new(app.query.clone())
        .block(Block::default().borders(Borders::ALL).title("Recherche"))
        .wrap(Wrap { trim: false });
    frame.render_widget(query, areas[1]);

    let filtered = app.filtered_decks();
    if filtered.is_empty() {
        let empty = Paragraph::new("Aucun deck ne correspond au filtre.")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("Decks"));
        frame.render_widget(empty, areas[2]);
    } else {
        let items = filtered
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let label = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("deck");
                let prefix = if index == app.selected_deck_index {
                    "▶ "
                } else {
                    "  "
                };
                ListItem::new(format!("{prefix}{label}"))
            })
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Decks"))
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(app.selected_deck_index));
        frame.render_stateful_widget(list, areas[2], &mut state);
    }

    let status = Paragraph::new(app.status.text.as_str())
        .style(style_for_tone(app.status.tone))
        .block(Block::default().borders(Borders::ALL).title("Statut"));
    frame.render_widget(status, areas[3]);

    let help = Paragraph::new("↑/↓ sélectionner · Entrée ouvrir · Échap reset filtre · q quitter")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, areas[4]);
}

fn draw_play_screen(frame: &mut Frame, app: &App) {
    let Some(session) = app.session.as_ref() else {
        return;
    };

    let (stage_len, total_cards, card_position) = session.progress();
    let show_question = should_show_question(stage_len, card_position);
    let question_text = if show_question {
        session.current_card().key.as_str()
    } else {
        "Carte rejouée, réponds de mémoire."
    };

    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(frame.area());

    let header = Paragraph::new(Line::from(vec![
        Span::styled("Deck ", Style::default().fg(Color::Cyan)),
        Span::styled(
            &app.deck_name,
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            " · Manche {} · Étape {stage_len}/{total_cards} · Carte {card_position}/{stage_len}",
            session.round()
        )),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Progression"));
    frame.render_widget(header, areas[0]);

    let card_title = if show_question {
        "Carte Active"
    } else {
        "Carte Mémorisée"
    };
    let card = Paragraph::new(question_text)
        .style(
            Style::default()
                .fg(if show_question {
                    Color::White
                } else {
                    Color::LightYellow
                })
                .add_modifier(if show_question {
                    Modifier::BOLD
                } else {
                    Modifier::ITALIC
                }),
        )
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title(card_title));
    frame.render_widget(card, areas[1]);

    let answer = Paragraph::new(app.answer_input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Ta Réponse"));
    frame.render_widget(answer, areas[2]);

    let status = Paragraph::new(app.status.text.as_str())
        .style(style_for_tone(app.status.tone))
        .block(Block::default().borders(Borders::ALL).title("Feedback"));
    frame.render_widget(status, areas[3]);

    let help = Paragraph::new("Entrée valider · Échap vider saisie · q quitter")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, areas[4]);
}

fn style_for_tone(tone: MessageTone) -> Style {
    match tone {
        MessageTone::Info => Style::default().fg(Color::Cyan),
        MessageTone::Success => Style::default().fg(Color::Green),
        MessageTone::Warning => Style::default().fg(Color::Yellow),
        MessageTone::Error => Style::default().fg(Color::Red),
    }
}

fn select_deck_from_selector(
    available_decks: &[PathBuf],
    selector: &str,
) -> Result<PathBuf, UiError> {
    let selector_path = PathBuf::from(selector);
    if selector_path.exists() {
        return Ok(selector_path);
    }

    let selector_lower = selector.to_lowercase();
    let matches = available_decks
        .iter()
        .filter(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| stem.to_lowercase())
                .map(|stem| stem == selector_lower || stem.contains(&selector_lower))
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Err(UiError::NoDeckMatches {
            needle: selector.to_string(),
        }),
        1 => Ok(matches[0].clone()),
        _ => Err(UiError::AmbiguousDeckMatch {
            needle: selector.to_string(),
        }),
    }
}

fn should_show_question(stage_len: usize, card_position: usize) -> bool {
    card_position == stage_len
}

#[cfg(test)]
mod tests {
    use super::should_show_question;

    #[test]
    fn replayed_cards_hide_question() {
        assert!(!should_show_question(3, 1));
        assert!(!should_show_question(3, 2));
        assert!(should_show_question(3, 3));
        assert!(should_show_question(1, 1));
    }
}
