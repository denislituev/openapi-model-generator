pub mod cli;
pub mod error;
pub mod generator;
pub mod models;
pub mod parser;

pub use error::Error;
pub use generator::{generate_models, GenerateMode};
pub use parser::parse_openapi;

pub type Result<T> = std::result::Result<T, Error>;
