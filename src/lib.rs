pub mod models;
pub mod generator;
pub mod parser;
pub mod cli;
pub mod error;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>; 