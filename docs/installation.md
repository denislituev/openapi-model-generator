# Installation

## Prerequisites

- **Rust toolchain** 1.70 or newer. Install via [rustup](https://rustup.rs/).

## CLI Tool

Install the `omg` binary directly from [crates.io](https://crates.io/crates/openapi-model-generator):

```bash
cargo install openapi-model-generator
```

Verify the installation:

```bash
omg --version
```

## Library Dependency

Add `openapi-model-generator` to `Cargo.toml` to use it programmatically:

```toml
[dependencies]
openapi-model-generator = "0.6.2"
```

### Minimal library example

```rust
use openapi_model_generator::{parse_openapi, generate_models, GenerateMode};
use std::fs;

fn main() -> anyhow::Result<()> {
    let content = fs::read_to_string("openapi.yaml")?;
    let openapi: openapiv3::OpenAPI = serde_yaml::from_str(&content)?;

    let (models, requests, responses) = parse_openapi(&openapi)?;
    let code = generate_models(&models, &requests, &responses, GenerateMode::ALL, false)?;

    fs::write("src/generated/models.rs", code.trim())?;
    Ok(())
}
```

## Building from Source

```bash
git clone https://github.com/denislituev/openapi-model-generator
cd openapi-model-generator
cargo build --release
# binary is at target/release/omg
```

## CLI Quick-Start

```bash
# Generate all types (models, requests, responses) from a YAML spec
omg -i path/to/openapi.yaml -o ./generated

# Generate only schema models
omg -i path/to/openapi.yaml -o ./generated -m models

# Generate models + request types
omg -i path/to/openapi.yaml -o ./generated -m requests

# Generate models + response types
omg -i path/to/openapi.yaml -o ./generated -m responses

# Also emit `impl Display` for every type
omg -i path/to/openapi.yaml -o ./generated --display
```

### CLI Parameters

| Flag | Short | Default | Description |
|---|---|---|---|
| `--input` | `-i` | *(required)* | Path to the OpenAPI spec (YAML or JSON) |
| `--output` | `-o` | `./generated` | Directory for generated files |
| `--mode` | `-m` | `all` | Generation mode: `models`, `requests`, `responses`, `all` |
| `--display` | | `false` | Generate `impl Display` for all types |

## Output Files

The tool writes two files into the output directory:

| File | Contents |
|---|---|
| `models.rs` | All generated Rust types |
| `mod.rs` | Module-level `use` re-exports and import declarations |
