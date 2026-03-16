use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Mode {
    /// Generate only models
    Models,

    /// Generate models and request types
    Requests,

    /// Generate models and response types
    Responses,

    /// Generate everything
    All,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long)]
    pub input: PathBuf,

    #[arg(short, long, default_value = "./generated")]
    pub output: PathBuf,

    /// Generation mode
    #[arg(short = 'm', long, value_enum, default_value_t = Mode::All)]
    pub mode: Mode,
}
