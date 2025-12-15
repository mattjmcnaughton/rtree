use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "rtree")]
#[command(about = "Print a deterministic ASCII directory tree", long_about = None)]
pub struct Cli {
    /// Root path to print (defaults to current directory)
    pub path: Option<PathBuf>,
}
