# Features

`openapi-model-generator` (`omg`) converts OpenAPI 3.0 specifications into type-safe Rust code.

## Input Formats

- **YAML** (`.yaml`) and **JSON** (`.json`) specification files are both supported.

## Schema Type Mapping

| OpenAPI type | Rust type |
|---|---|
| `string` | `String` |
| `string` + `format: uuid` | `Uuid` |
| `string` + `format: date-time` | `DateTime<Utc>` |
| `integer` / `number` | `i64` / `f64` |
| `boolean` | `bool` |
| `array` | `Vec<T>` |
| `object` | custom `struct` |
| `string` with `enum` values | Rust `enum` |

## Schema Composition

| OpenAPI keyword | Generated Rust construct |
|---|---|
| `allOf` | Flat `struct` merging all fields from referenced schemas |
| `oneOf` | Tagged-union `enum` (internally untagged via serde) |
| `anyOf` | Tagged-union `enum` (same as `oneOf`) |

## Field Handling

- Required vs. optional fields are detected automatically; optional fields become `Option<T>`.
- Duplicate fields produced by `allOf` merging are deduplicated — concrete types (e.g. `i64`) are preferred over `serde_json::Value`.
- `additionalProperties` is exposed as a flattened `HashMap` field.

## Request & Response Model Generation

- Models for `components/schemas` are always generated.
- Request body models from `components/requestBodies` and inline request/response bodies on path operations are generated when the corresponding mode is active.
- Generated names for request/response types follow **PascalCase** convention derived from the operation id.

## Generation Modes

| Mode flag | What is included |
|---|---|
| `models` | `components/schemas` only |
| `requests` | schemas + request body types |
| `responses` | schemas + response types |
| `all` (default) | schemas + requests + responses |

## OpenAPI Extension Support

| Extension | Effect |
|---|---|
| `x-rust-type` | Replaces the generated type with a custom Rust type alias |
| `x-rust-attrs` | Injects additional `#[…]` attributes on the generated type or field |

Both extensions work on schema objects and on individual properties.

## Display Implementations

Pass `--display` to generate `impl std::fmt::Display` for every type:

- Enums and unions render their serde-rename value (e.g. `"config_sync"`).
- Structs and compositions fall back to `{:?}` (Debug format).
- Types that already include a `Display` derive via `x-rust-attrs` are skipped.

## Validation Attribute Generation

The generator emits field-level validation attributes compatible with the [validator](https://docs.rs/validator) crate.

### Numeric range validation

OpenAPI `minimum` / `maximum` constraints produce `#[validate(range(...))]` attributes.

- Fields typed as `f64` or `f32` use **float literals**: `#[validate(range(min = 0.0, max = 1.0))]`.
- Integer fields (`i64`, `i32`, etc.) use integer literals: `#[validate(range(min = 0, max = 100))]`.

### Regex pattern validation

OpenAPI `pattern` constraints produce a `LazyLock<Regex>` static and a `#[validate(regex(...))]` attribute:

```rust
use std::sync::LazyLock;
use regex::Regex;

static RE_1: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z]+$").unwrap());

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct MyModel {
    #[validate(regex(path = RE_1))]
    pub code: String,
}
```

Identical patterns across multiple fields share a single static (deduplicated by pattern string).

> **Note:** Specs that contain `pattern` constraints require the `regex` crate in the consumer's `Cargo.toml`:
> ```toml
> [dependencies]
> regex = "1"
> ```

## Code Quality

- All generated code includes proper `use` imports (`uuid`, `chrono`, `serde`, etc.).
- Rust reserved keywords used as field names are automatically escaped with `r#`.
- A generated file header records the tool name and version used.
