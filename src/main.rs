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
            Commands::Heartbeat {
                fetch,
                trends,
                insights,
                status,
            } => {
                use turbo_search::heartbeat::{config::FetcherConfig, scheduler::HeartbeatScheduler};
                use tokio::runtime::Runtime;

                let rt = Runtime::new()?;
                rt.block_on(async {
                    let mut scheduler = HeartbeatScheduler::new(FetcherConfig::default());

                    if fetch {
                        match scheduler.fetch_now().await {
                            Ok(insights) => {
                                println!("Fetched {} insights:", insights.len());
                                for (i, insight) in insights.iter().take(10).enumerate() {
                                    println!("{}. [{}] {}", i + 1, insight.source, insight.title);
                                }
                            }
                            Err(e) => {
                                eprintln!("Fetch failed: {e}");
                            }
                        }
                    } else if trends {
                        println!("Use --fetch to get trends first");
                    } else if insights {
                        println!("Use --fetch to get insights first");
                    } else if status {
                        let stats = scheduler.get_stats().await;
                        println!("Heartbeat Status:");
                        println!("  Total fetches: {}", stats.total_fetches);
                        println!("  Successful: {}", stats.successful_fetches);
                        println!("  Failed: {}", stats.failed_fetches);
                    } else {
                        println!("Heartbeat commands:");
                        println!("  --fetch    Fetch latest skills");
                        println!("  --trends   Show trends");
                        println!("  --insights Show insights");
                        println!("  --status   Show status");
                    }
                });
            }
        }
    }

    Ok(())
}
