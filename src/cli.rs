use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rtree")]
#[command(about = "Print a deterministic ASCII directory tree", long_about = None)]
pub struct Cli {
    /// Root path to print (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Limit directory traversal to specified depth
    #[arg(short = 'L')]
    pub level: Option<usize>,

    /// Ignore files/directories matching pattern (pipe-separated, e.g., "node_modules|.git")
    #[arg(short = 'I')]
    pub ignore_pattern: Option<String>,

    /// Show all files including hidden (currently the default behavior)
    #[arg(short = 'a')]
    pub all: bool,

    /// List directories only
    #[arg(short = 'd')]
    pub dirs_only: bool,

    /// List directories before files
    #[arg(long = "dirsfirst")]
    pub dirs_first: bool,
}
