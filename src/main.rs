use std::path::PathBuf;

use clap::Parser;
use squizz_it::{
    config::{AppConfig, PlayMode},
    ui::UiOptions,
};

#[derive(Debug, Parser)]
#[command(name = "squizz-it")]
#[command(about = "Application d'apprentissage à base de flashcards CSV")]
struct Cli {
    #[arg(long, default_value = "squizz-it.toml")]
    config: PathBuf,
    #[arg(long)]
    deck: Option<String>,
    #[arg(long)]
    search: Option<String>,
    #[arg(long)]
    seed: Option<u64>,
    /// Launch a review session with only due cards.
    #[arg(long)]
    review: bool,
    /// Play mode: simon, classic, or reverse.
    #[arg(long, default_value = "simon")]
    mode: PlayMode,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut config = AppConfig::load_or_default(&cli.config)?;
    if let Some(seed) = cli.seed {
        config.game.shuffle_seed = Some(seed);
    }

    squizz_it::ui::run(
        &config,
        UiOptions {
            deck: cli.deck,
            search: cli.search,
            review: cli.review,
            play_mode: Some(cli.mode),
        },
    )?;

    Ok(())
}
