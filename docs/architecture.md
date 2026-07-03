# Architecture

## Repository Layout

```
openapi-model-generator/
├── Cargo.toml          # Package manifest and dependencies
├── src/
│   ├── main.rs         # CLI entry point
│   ├── lib.rs          # Public library surface
│   ├── cli.rs          # Argument definitions (clap)
│   ├── parser.rs       # OpenAPI → internal model conversion
│   ├── models.rs       # Internal data model types
│   ├── generator.rs    # Internal model → Rust source code
│   └── error.rs        # Error type definitions
└── docs/               # Project documentation
```

## Module Responsibilities

### `cli` — Command-Line Interface

Defines the `Args` struct (parsed by [clap](https://docs.rs/clap)) and the `Mode` enum:

- `Mode::Models` → `GenerateMode::MODELS`
- `Mode::Requests` → `GenerateMode::MODELS | GenerateMode::REQUESTS`
- `Mode::Responses` → `GenerateMode::MODELS | GenerateMode::RESPONSES`
- `Mode::All` → `GenerateMode::ALL`

### `error` — Unified Error Type

A single `Error` enum built with [thiserror](https://docs.rs/thiserror):

| Variant | Source |
|---|---|
| `Io` | `std::io::Error` |
| `Yaml` | `serde_yaml::Error` |
| `Json` | `serde_json::Error` |
| `OpenApi` | Logic errors during OpenAPI traversal |
| `Generation` | Logic errors during code generation |

### `models` — Internal Data Model

Strongly-typed structs that represent parsed OpenAPI schemas before code generation. Key types:

| Type | Represents |
|---|---|
| `ModelType` | Top-level discriminated union over all model kinds |
| `Model` | A plain `struct` with named fields |
| `UnionModel` | A `oneOf`/`anyOf` tagged-union `enum` |
| `CompositionModel` | An `allOf` struct with merged fields |
| `EnumModel` | A string schema with `enum` values |
| `TypeAliasModel` | An `x-rust-type` type alias |
| `Field` | A single struct field (name, type, nullability, validation rules, custom attrs) |
| `RequestModel` / `ResponseModel` | Request/response body wrappers |
| `ValidationRules` | OpenAPI numeric/string constraints (min, max, pattern, …) |

### `parser` — OpenAPI → Internal Models (`parse_openapi`)

Entry point: `pub fn parse_openapi(openapi: &OpenAPI) -> Result<(Vec<ModelType>, Vec<RequestModel>, Vec<ResponseModel>)>`

Processing steps:
1. **Schema traversal** — iterates `components.schemas`, resolves `$ref` references.
2. **Type inference** — maps OpenAPI primitive types and formats to Rust types.
3. **Composition handling** — detects `allOf`, `oneOf`, `anyOf` and builds the corresponding internal model.
4. **Extension extraction** — reads `x-rust-type` and `x-rust-attrs` from `schema.extensions`.
5. **Path traversal** — iterates `paths` to collect request body and response schemas.
6. **Name normalisation** — converts operation ids and schema names to PascalCase via `to_pascal_case`.
7. **Deduplication** — uses a `HashSet` to avoid emitting the same model twice.

### `generator` — Internal Models → Rust Source (`generate_models`)

Entry point: `pub fn generate_models(models, requests, responses, mode: GenerateMode, display: bool) -> Result<String>`

Processing steps:
1. **Mode filtering** — skips request/response types when the mode flags are not set.
2. **Use-statement detection** — scans every generated type to decide whether `uuid`, `chrono`, or other imports are needed (`RequiredUses` bitflags).
3. **Per-type code generation** — dispatches to a dedicated function for each `ModelType` variant:
   - `generate_struct` for `Model`
   - `generate_union` for `UnionModel`
   - `generate_composition` for `CompositionModel`
   - `generate_enum` for `EnumModel`
   - `generate_type_alias` for `TypeAliasModel`
4. **Attribute injection** — emits `#[derive(…)]`, `#[serde(…)]`, and any `x-rust-attrs` attributes.
5. **Display generation** — optionally appends `impl std::fmt::Display` blocks.
6. **Header** — prepends a doc comment recording the tool name and version.

`generate_lib` produces the companion `mod.rs` file with the necessary `use` re-exports.

## Data Flow

```
OpenAPI YAML/JSON file
        │
        ▼
  serde_yaml / serde_json
        │   (deserialise into openapiv3::OpenAPI)
        ▼
   parser::parse_openapi()
        │   (produces Vec<ModelType>, Vec<RequestModel>, Vec<ResponseModel>)
        ▼
 generator::generate_models()
        │   (produces Rust source as a String)
        ▼
  fs::write("models.rs")
  fs::write("mod.rs")
```

## Key Dependencies

| Crate | Role |
|---|---|
| [openapiv3](https://docs.rs/openapiv3) | Deserialise OpenAPI 3.0 documents |
| [serde](https://docs.rs/serde) + [serde_yaml](https://docs.rs/serde_yaml) + [serde_json](https://docs.rs/serde_json) | Serialisation / deserialisation |
| [clap](https://docs.rs/clap) | CLI argument parsing |
| [thiserror](https://docs.rs/thiserror) | Ergonomic error definitions |
| [indexmap](https://docs.rs/indexmap) | Ordered maps for deterministic field ordering |
| [bitflags](https://docs.rs/bitflags) | `GenerateMode` and `RequiredUses` flag sets |
| [tracing](https://docs.rs/tracing) | Diagnostic logging |
