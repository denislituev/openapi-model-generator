use openapi_model_generator::{cli::Args, parser, generator, Result, Error};
use std::fs;
use openapiv3::OpenAPI;
use clap::Parser;
use std::io;
use std::path::Path;

pub fn validate_input_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    println!("Checking input file: {:?}", path);

    if !path.exists() {
        return Err(Error::from(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Input path {:?} does not exist", path),
        )));
    }

    if !path.is_file() {
        return Err(Error::from(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Input path {:?} is not a file", path),
        )));
    }

    fs::File::open(path).map(|_| {
        println!("Input file is valid and readable.");
    })?;

    Ok(())
}
pub fn create_output_dir<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    println!("Checking output directory: {:?}", path);

    if path.exists() {
        if path.is_dir() {
            println!("Output directory already exists.");
            Ok(())
        } else {
            Err(Error::from(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("Path {:?} exists but is not a directory", path),
            )))
        }
    } else {
        println!("Creating directory: {:?}", path);
        fs::create_dir_all(path)?;
        println!("Directory created.");
        Ok(())
    }
}


#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if let Err(e) = validate_input_file(&args.input) {
        eprintln!("Failed to validate input file: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = create_output_dir(&args.output) {
        eprintln!("Failed to create output directory: {}", e);
        std::process::exit(1);
    }
    
    let content = fs::read_to_string(&args.input)?;

    let openapi: OpenAPI = if args.input.extension().map_or(false, |ext| ext == "yaml") {
        serde_yaml::from_str(&content)?
    } else {
        serde_json::from_str(&content)?
    };

    let models = parser::parse_openapi(&openapi)?;

    let rust_code = generator::generate_rust_code(&models)?;
    let output_models_path = args.output.join("models.rs");
    fs::write(&output_models_path, rust_code)?;

    let rust_lib = generator::generate_lib()?;
    let output_lib_path = args.output.join("mod.rs");
    fs::write(&output_lib_path, rust_lib)?;

    println!("Models generated successfully to {:?}", output_models_path);

    Ok(())
}
