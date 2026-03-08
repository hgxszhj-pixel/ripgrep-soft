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
        /// Path to search in (file or directory)
        #[arg(short, long, default_value = ".")]
        path: String,

        /// Search pattern for filename search
        #[arg(long)]
        pattern: Option<String>,

        /// Content search pattern (search inside files)
        #[arg(short = 'c', long = "content")]
        content: Option<String>,

        /// Use regex for pattern matching
        #[arg(long, short = 'e')]
        regex: bool,

        /// Use glob pattern matching (e.g., *.mp4, *.txt)
        #[arg(long, short = 'g')]
        glob: bool,

        /// Case sensitive search
        #[arg(long, short = 'i')]
        case_sensitive: bool,

        /// Number of lines of context around matches
        #[arg(long = "context", short = 'C', default_value = "0")]
        context: usize,
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
