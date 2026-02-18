mod app;
mod app_log;
mod command;
mod event;
mod keybindings;
mod panes;
mod resource_switcher;
mod state;

use std::io;

use clap::Parser;
use crossterm::execute;
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::app::App;
use crate::keybindings::KeybindingDispatcher;

#[derive(Parser)]
#[command(name = "crystal", about = "Keyboard-driven Kubernetes TUI IDE")]
struct Cli {
    /// Generate default config file at ~/.config/crystal/config.toml
    #[arg(long)]
    init_config: bool,

    /// Print effective config (defaults + user overrides) and exit
    #[arg(long)]
    print_config: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.init_config {
        let path = crystal_config::AppConfig::init_default()?;
        println!("Config written to {}", path.display());
        return Ok(());
    }

    if cli.print_config {
        let config = crystal_config::AppConfig::load();
        println!("{}", toml::to_string_pretty(&config)?);
        return Ok(());
    }

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).with_writer(crate::app_log::AppLogMakeWriter).init();

    install_panic_hook();

    terminal::enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let config = crystal_config::Config::load();
    let dispatcher = KeybindingDispatcher::from_config(&config.keybindings);
    let theme = crystal_tui::theme::Theme::from_config(&config.theme);
    let mut app = App::new(config.tick_rate_ms(), dispatcher, theme, config.views).await;
    let result = app.run(&mut terminal).await;

    terminal::disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    result
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
}
