use clap::{Parser, ValueEnum};
use std::path::PathBuf;

use crate::generator::GenerateMode;

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

impl From<Mode> for GenerateMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Models => GenerateMode::MODELS,
            Mode::Requests => GenerateMode::MODELS | GenerateMode::REQUESTS,
            Mode::Responses => GenerateMode::MODELS | GenerateMode::RESPONSES,
            Mode::All => GenerateMode::ALL,
        }
    }
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

    /// Generate `impl std::fmt::Display` for all types.
    /// Enums and unions display their serde-rename value; structs and compositions
    /// fall back to `{:?}` (Debug). Off by default.
    #[arg(long, default_value_t = false)]
    pub display: bool,
}
