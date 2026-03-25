use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "turbo-search")]
#[command(about = "A high-performance file and content search tool", long_about = None)]
pub struct Cli {
    /// Launch GUI mode (default when no subcommand is provided)
    #[arg(long, short)]
    pub gui: bool,

    #[arg(global = true, short, long)]
    pub verbose: bool,

    #[arg(global = true, short, long)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Search {
        /// Path to search in (file or directory)
        #[arg(short, long)]
        path: Option<String>,

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

        /// Limit number of results
        #[arg(long, short = 'l', default_value = "100")]
        limit: usize,
    },
    Index {
        #[arg(short, long)]
        path: Option<String>,

        #[arg(short, long)]
        rebuild: bool,
    },
    /// Heartbeat: Fetch latest skills from GitHub and Reddit
    Heartbeat {
        /// Fetch from all sources
        #[arg(long, short)]
        fetch: bool,

        /// Show current trends
        #[arg(long, short)]
        trends: bool,

        /// Show latest insights
        #[arg(long, short)]
        insights: bool,

        /// Show heartbeat status
        #[arg(long, short)]
        status: bool,
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

    /// Returns true if GUI mode should be launched
    pub fn should_launch_gui(&self) -> bool {
        // GUI if --gui flag is set, or no command provided
        self.gui || self.command.is_none()
    }
}
