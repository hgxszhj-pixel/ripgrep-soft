use clap::Parser;
use turbo_search::cli::{Cli, Commands};
use turbo_search::cli_search;
use turbo_search::gui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    turbo_search::logging::init(cli.log_level())?;

    // Check if we should launch GUI or run CLI command
    if cli.should_launch_gui() {
        gui::run_gui()?;
    } else if let Some(command) = cli.command {
        match command {
            Commands::Search {
                path,
                pattern,
                content,
                regex,
                glob,
                case_sensitive,
                context,
                limit,
            } => {
                cli_search::run_search(
                    path,
                    pattern,
                    content,
                    regex,
                    glob,
                    case_sensitive,
                    context,
                    limit,
                )?;
            }
            Commands::Index { path, rebuild } => {
                cli_search::run_index(path, rebuild)?;
            }
        }
    }

    Ok(())
}
