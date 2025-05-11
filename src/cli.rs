use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Путь к файлу OpenAPI спецификации (YAML или JSON)
    #[arg(short, long)]
    pub input: PathBuf,

    /// Путь к выходной директории
    #[arg(short, long, default_value = "./generated")]
    pub output: PathBuf,
} 