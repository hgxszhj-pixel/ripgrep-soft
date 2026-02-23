use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ripgrep-soft")]
#[command(about = "A high-performance file and content search tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(global = true, short, long)]
    pub verbose: bool,

    #[arg(global = true, short, long)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    Search {
        #[arg(short, long, default_value = ".")]
        path: String,

        #[arg(long)]
        pattern: Option<String>,

        #[arg(long)]
        content: Option<String>,

        #[arg(long)]
        regex: bool,
    },
    Index {
        #[arg(short, long, default_value = ".")]
        path: String,

        #[arg(short, long)]
        rebuild: bool,
    },
}

impl Cli {
    pub fn log_level(&self) -> &str {
        if self.quiet {
            "warn"
        } else if self.verbose {
            "debug"
        } else {
            "info"
        }
    }
}
