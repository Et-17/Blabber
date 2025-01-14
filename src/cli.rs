use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version, about)]
pub struct Cli {
    /// File containing the grammar
    pub file: PathBuf,

    /// Start symbol (default: first in the file)
    #[arg(short, long, value_name = "SYMBOL")]
    pub start: Option<String>,

    /// Amount to generate (default: 1)
    #[arg(short = 'n', long, value_name = "AMOUNT")]
    pub amount: Option<u32>
}