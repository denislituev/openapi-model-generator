# OpenAPI Model Generator

A Rust library and CLI tool for generating Rust models from OpenAPI specifications. This utility automatically creates Rust structures based on schemas from OpenAPI (Swagger) documentation.

## Features

- **OpenAPI 3.0 specification support** with full schema parsing
- **YAML and JSON format support** for input specifications
- **Automatic generation of Rust structures** with Serde attributes
- **Schema Composition Support**: Complete implementation of OpenAPI composition patterns:
  - `allOf` - Combines multiple schemas into a single struct
  - `oneOf` / `anyOf` - Generates tagged union enums with proper serde configuration
- **Advanced Type Support**: 
  - Enum Support - Automatically generates Rust enums from OpenAPI string schemas with enumeration constraints.
  - UUID fields (`format: uuid` → `Uuid` type)
  - DateTime fields (`format: date-time` → `DateTime<Utc>` type)
  - Nested types and arrays with proper generic handling
- **Custom Type Support**:
  - `x-rust-type` extension - Replace generated models with custom Rust types (type aliases)
  - `x-rust-attrs` extension - Add custom Rust attributes to generated types
  - Works with any schema type (object, enum, oneOf, etc.)
  - Support for `x-rust-type` on individual properties
- **Smart Field Deduplication**: Automatically resolves duplicate field names in `allOf` compositions
  - Preserves concrete types (e.g., `i64`, `String`) over generic `serde_json::Value`
  - Prevents compilation errors from duplicate struct fields
- **Array Composition Support**: Full support for arrays with complex item types
  - Arrays with `oneOf` items → `Vec<UnionEnum>`
  - Arrays with any schema composition pattern
- **Request Bodies Support**: Full parsing and model generation from `components.requestBodies`
- **Smart Code Generation**:
  - Required vs optional field detection (`Option<T>` for nullable fields)
  - PascalCase naming for generated request/response models
  - Reference resolution across schema definitions
- **Clean Code Output**: Properly formatted Rust code with comprehensive serde annotations

## Installation

### As a CLI tool

```bash
cargo install openapi-model-generator
```

### As a library dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
openapi-model-generator = "0.6.2"
```

## Usage

### Command Line Interface

```bash
# Generate everything (default)
omg -i path/to/openapi.yaml -o ./generated

# Generate only models
omg -i path/to/openapi.yaml -o ./generated -m models

# Generate models and request types
omg -i path/to/openapi.yaml -o ./generated -m requests

# Generate models and response types
omg -i path/to/openapi.yaml -o ./generated -m responses

# Generate with Display implementations
omg -i spec.yaml -o ./generated --display
```

### Parameters

- `-i, --input` - Path to the OpenAPI specification file (YAML or JSON)
- `-o, --output` - Path to the output directory (default: ./generated)
- `-m, --mode <MODE>` - Generation mode (default: all)
  - `models` - Generate only models from `components/schemas`
  - `requests` - Generate models and request types from API endpoints
  - `responses` - Generate models and response types from API endpoints
  - `all` - Generate everything (models, requests, and responses)
- `--display` - Generate `impl std::fmt::Display` for all types (default: off)
  - Enums and unions display their serde-rename value (e.g. `"config_sync"`)
  - Structs and compositions fall back to `{:?}` (Debug)
  - Types that already include a `Display` derive in `x-rust-attrs` are skipped

### Validation dependencies

Generated code with validation rules (`minLength`, `maximum`, `pattern`, etc.) uses the [`validator`](https://crates.io/crates/validator) crate. Add to your `Cargo.toml`:

```toml
[dependencies]
validator = { version = "0.20", features = ["derive"] }
```

When the spec contains `pattern` constraints, the generated code also uses the [`regex`](https://crates.io/crates/regex) crate:

```toml
regex = "1"
```

### Library Usage

```rust
use openapi_model_generator::{parse_openapi, generate_models, GenerateMode};
use std::fs;

// Parse OpenAPI specification
let openapi_spec = fs::read_to_string("openapi.yaml")?;
let openapi: openapiv3::OpenAPI = serde_yaml::from_str(&openapi_spec)?;

// Generate models
let (models, requests, responses) = parse_openapi(&openapi)?;
let generated_code = generate_models(&models, &requests, &responses, GenerateMode::ALL, false)?;

// Write to file
fs::write("models.rs", generated_code)?;
```

## Example

Source OpenAPI schema:
```yaml
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
        email:
          type: string
        age:
          type: integer
        is_active:
          type: boolean
      required:
        - id
        - name
        - email
```

Generated Rust code:
```rust
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, NaiveDate, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "id")]
    pub id: Uuid,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "email")]
    pub email: String,
    #[serde(rename = "age")]
    pub age: Option<i64>,
    #[serde(rename = "is_active")]
    pub is_active: Option<bool>,
}
```

### Using Custom Types with `x-rust-type`

You can use the `x-rust-type` extension to replace generated models with your own custom types:

```yaml
components:
  schemas:
    User:
      type: object
      x-rust-type: crate::domain::User
      description: "Custom domain user type"
      properties:
        id:
          type: string
          format: uuid
        name:
          type: string
    
    Status:
      type: string
      enum: [active, inactive, pending]
      x-rust-type: common::enums::Status
```

Generated Rust code:
```rust
/// Custom domain user type
pub type User = crate::domain::User;

/// Status
pub type Status = common::enums::Status;
```

This allows you to:
- Reuse existing domain models instead of generating duplicates
- Integrate with types from other crates
- Maintain a clean separation between API models and domain models

### Using Custom Attributes with `x-rust-attrs`

You can use the `x-rust-attrs` extension to add arbitrary Rust attributes to generated types:

```yaml
components:
  schemas:
    User:
      type: object
      x-rust-attrs:
        - "#[serde(rename_all = \"camelCase\")]"
      properties:
        user_id:
          type: string
          format: uuid
        first_name:
          type: string
        is_active:
          type: boolean
      required:
        - user_id
        - first_name
    
    Status:
      type: string
      enum: [ACTIVE, INACTIVE, PENDING]
      x-rust-attrs:
        - "#[serde(rename_all = \"UPPERCASE\")]"
    
    Product:
      type: object
      x-rust-attrs:
        - "#[derive(Serialize, Deserialize)]"
        - "#[serde(deny_unknown_fields)]"
      properties:
        id:
          type: string
        name:
          type: string
```

Generated Rust code:
```rust
/// User
#[serde(rename_all = "camelCase")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: Uuid,
    pub first_name: String,
    pub is_active: Option<bool>,
}

/// Status
#[serde(rename_all = "UPPERCASE")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Status {
    ACTIVE,
    INACTIVE,
    PENDING
}

/// Product
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: Option<String>,
    pub name: Option<String>,
}
```

This allows you to:
- Add custom serde rules (rename_all, deny_unknown_fields, tag, etc.)
- Enable additional trait derives
- Apply conditional compilation attributes (cfg, cfg_attr)
- Use custom validation macros
- Works together with `x-rust-type` extension
- **Note**: `x-rust-attrs` can also be applied to individual struct fields for fine-grained control.

## Recent Updates (v0.6.0)

- **Added**: Validation generation support from OpenAPI schema constraints
- **Added**: CLI generation mode support to control what types of code are generated
- **Added**: `x-rust-attrs` extension support on individual struct fields
- **Added**: Support for `gen` keyword as described in RFC3513
- **Fixed**: Double Vec wrapping bug for fields referencing array type aliases via `$ref`
- **Fixed**: Missing chrono imports when structs contain `DateTime<Utc>` fields
- **Improved**: Library API to support generation modes (models, requests, responses, all)

## Previous Updates (v0.5.0)

- **Added**: Support for `x-rust-type` extension on individual properties
- **Improved**: Multi-line description formatting for better documentation
- **Fixed**: Duplicate field names in `allOf` compositions with different types
- **Added**: Support for arrays with `oneOf` items
- **Improved**: Enum variant names automatically converted to PascalCase
- **Added**: Module-level documentation comments in generated files
- **Improved**: Code quality - removed non-English comments, fixed clippy warnings

## Development

### Dependencies

- Rust 1.70 or higher
- Cargo

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

## Examples

The generator supports complex OpenAPI patterns including schema composition:

- **UUID and DateTime handling** - Automatic type conversion for formatted strings
- **Schema composition with allOf** - Inheritance and field merging 
- **Tagged unions with oneOf/anyOf** - Automatic enum generation with proper serde tags

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for detailed information about releases and changes.

## License

MIT
