use clap::Parser;
use openapi_model_generator::{cli::Args, generator, parser, Error, Result};
use openapiv3::OpenAPI;
use std::fs;
use std::io;
use std::path::PathBuf;

pub fn validate_input_file(path: &PathBuf) -> Result<()> {
    println!("Checking input file: {path:?}");

    if !path.exists() {
        return Err(Error::from(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Input path {path:?} does not exist"),
        )));
    }

    if !path.is_file() {
        return Err(Error::from(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Input path {path:?} is not a file"),
        )));
    }

    fs::File::open(path).map(|_| {
        println!("Input file is valid and readable.");
    })?;

    Ok(())
}

pub fn create_output_dir(path: &PathBuf) -> Result<()> {
    println!("Checking output directory: {path:?}");

    if path.exists() {
        if path.is_dir() {
            println!("Output directory already exists.");
            Ok(())
        } else {
            Err(Error::from(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Path {path:?} exists but is not a directory"),
            )))
        }
    } else {
        println!("Creating directory: {path:?}");
        fs::create_dir_all(path)?;
        println!("Directory created.");
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    if let Err(e) = validate_input_file(&args.input) {
        eprintln!("Failed to validate input file: {e}");
        std::process::exit(1);
    }

    if let Err(e) = create_output_dir(&args.output) {
        eprintln!("Failed to create output directory: {e}");
        std::process::exit(1);
    }

    let content = fs::read_to_string(&args.input)?;

    let openapi: OpenAPI = if args.input.extension().is_some_and(|ext| ext == "yaml") {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    let (models, requests, responses) = parser::parse_openapi(&openapi)?;

    let rust_code = generator::generate_models(
        &models,
        &requests,
        &responses,
        args.mode.into(),
        args.display,
    )?;
    let output_models_path = args.output.join("models.rs");
    fs::write(&output_models_path, rust_code.trim())?;

    let rust_lib = generator::generate_lib()?;
    let output_lib_path = args.output.join("mod.rs");
    fs::write(&output_lib_path, rust_lib.trim())?;

    println!("Models generated successfully to {output_models_path:?}");

    Ok(())
}
