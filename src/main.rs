use openapi_model_generator::{cli::Args, parser, generator, Result};
use std::fs;
use openapiv3::OpenAPI;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    fs::create_dir_all(&args.output)?;
    
    let content = fs::read_to_string(&args.input)?;

    let openapi: OpenAPI = if args.input.extension().map_or(false, |ext| ext == "yaml") {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    let models = parser::parse_openapi(&openapi)?;

    let rust_code = generator::generate_rust_code(&models)?;

    let output_path = args.output.join("models.rs");
    fs::write(&output_path, rust_code)?;

    println!("Models generated successfully to {:?}", output_path);

    Ok(())
}
