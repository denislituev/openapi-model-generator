# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.2] - 2026-05-24

### Added
- **`--display` CLI flag**: Opt-in generation of `impl std::fmt::Display` for enums, unions, structs, and compositions. Uses serde-rename values for enum/union variants and `Debug` as a fallback for structs. Skipped if `x-rust-attrs` already includes a custom `Display` derive.

### Fixed
- **Validator trait name**: Fixed wrong trait name in generated imports — `validator::Validator` → `validator::Validate`.

- **Validate derive placement**: Replaced fragile single-pass struct generation with a two-pass approach. The `#[derive(Validate)]` line is now emitted only after examining all fields, eliminating incorrect positioning when validation attributes are present.

- **Removed debug tracing**: Removed two `tracing::info!` calls that dumped internal model state in `generate_request_model`.

### Tests
- Added `test_schema_level_nullable_ref_makes_field_optional` — verifies nullable propagation via `$ref`.
- Added `test_inline_nullable_field_is_optional` — verifies nullable propagation on inline fields.

## [0.6.1] - 2026-05-22

### Fixed
- **Inline Enum Name Collision**: Fixed a bug where multiple structs with inline string enums on fields sharing the same name (e.g. `type`) produced colliding enum names. Inline enum names are now qualified with the parent struct name (e.g. `SignalAType`, `SignalBType`).

- **x-rust-attrs on Inline Enum Fields**: Fixed `x-rust-attrs` leaking from inline enum fields into the parent struct as invalid field-level attributes. Custom attributes now stay on the generated enum type only.

- **allOf Primitive Narrowing**: Fixed a bug where a base schema field declared as a primitive (`String`, `i64`, etc.) was not replaced by a more specific type (e.g. an inline string enum) from a composing allOf schema.

- **x-rust-attrs from $ref Target**: Fixed `x-rust-attrs` from a referenced schema being incorrectly propagated to the referencing field, producing invalid Rust (e.g. `#[derive(...)]` inside a struct body).

## [0.6.0] - 2026-03-22

### Added
- **Validation Generation**: Added support for generating field validations from OpenAPI schema constraints. The generator now creates validation structs that can be used to validate incoming data against schema requirements.

- **CLI Generation Mode**: Added CLI options to control what types of Rust code are generated. Use `-m` flag to choose:
  - `models` - Generate only models from `components/schemas`
  - `requests` - Generate models and request types from API endpoints
  - `responses` - Generate models and response types from API endpoints
  - `all` - Generate everything (default)

- **x-rust-attrs on Struct Fields**: Extended `x-rust-attrs` extension support to individual struct fields, allowing custom attributes like custom derives, serde rules, or conditional compilation attributes on specific fields.

- **gen Keyword Support**: Added support for the `gen` keyword as described in RFC3513, providing more granular control over which types are included in the generated output.

### Fixed
- **Double Vec Wrapping**: Fixed a critical bug where fields that reference named array type aliases via `$ref` were being incorrectly double-wrapped in `Vec<>`. The generator now correctly uses the referenced type name as-is without adding unnecessary wrappers.

- **Chrono Imports**: Fixed missing chrono imports in generated code when structs contain `DateTime<Utc>` fields. The generator now properly includes `use chrono::{DateTime, Utc};` when needed.

### Changed
- **Library API**: Enhanced the library API to support generation modes, allowing library users to specify what types of code they want to generate (models, requests, responses, or all).

## [0.5.2] - 2026-02-16

### Fixed
- **Stable Field Order**: Replaced `HashMap` with `IndexMap` in `resolve_all_of_fields()` function to maintain consistent field ordering in generated models. This ensures deterministic output across multiple invocations, fixing issue #44.

## [0.5.1] - 2026-02-13

### Fixed
- **Smart Import Tracking**: Fixed import generation to use bitflags for accurate type tracking instead of string-based detection. This ensures imports (uuid, chrono, serde_json::Value) are only included when actually needed in generated code.
- **Enhanced allOf Type Replacement**: Improved the resolver's ability to replace serde_json::Value with concrete types in allOf compositions. Now handles:
  - `Option<serde_json::Value>`
  - `HashMap<String, serde_json::Value>`
  - `Option<HashMap<String, serde_json::Value>>`
  - `Vec<serde_json::Value>`
  - `Option<Vec<serde_json::Value>>`

### Changed
- Removed unused `tokio-test` dependency
- Added `bitflags` dependency for efficient flag-based type tracking

## [0.5.0] - 2025-02-02

### Added
- **x-rust-type on Properties**: Support for `x-rust-type` extension on individual properties, not just on schemas. This allows using custom Rust types for specific fields while keeping others auto-generated.

- **Enhanced Description Formatting**: Multi-line descriptions are now properly formatted as separate doc comment lines instead of being on a single line. This improves readability for complex schemas with detailed documentation.

- **Block File Comments**: Added module-level documentation comments at the top of generated files indicating the generator name and version.

### Fixed
- **allOf Field Deduplication**: Resolved issue where `allOf` compositions with overlapping field names generated duplicate struct fields. The parser now:
  - Uses `HashMap` for field deduplication instead of `Vec`
  - Preserves concrete types (e.g., `i64`, `String`) over generic `serde_json::Value`
  - Prevents Rust compilation errors from duplicate struct fields
  - Correctly handles field type overriding when schemas have the same field name

- **Array with oneOf Items**: Full support for arrays where items use `oneOf` composition. The generator now:
  - Creates union enums for array items with multiple types
  - Generates `Vec<UnionEnum>` type aliases for such arrays
  - Properly handles complex array schemas with nested compositions

- **Enum Variant Naming**: Enum variants are now automatically converted to PascalCase, preventing Rust compiler warnings about non-standard naming.

### Changed
- **Code Quality**: Removed all Russian comments from source code for better international collaboration and maintainability.

- **Clippy Compliance**: Fixed `let_and_return` warning in `src/parser.rs` by returning expression directly instead of using unnecessary let binding.

## [0.4.1] - 2025-12-05

### Fixed
- **Additional Properties Support**: Proper generation of `HashMap<String, T>` for OpenAPI objects with `additionalProperties` instead of fallback to `serde_json::Value`
- **Empty Object Handling**: Generate empty structs for objects without properties instead of skipping them entirely
- **Import Optimization**: Remove unused imports (chrono::NaiveDate, uuid) from generated code when not required by the models

## [0.4.0] - 2025-11-13

### Added
- **Request Bodies Support**: Full parsing and model generation from `components.requestBodies`. The generator now extracts schemas from request bodies and creates appropriate Rust models, supporting both inline schemas and `$ref` references.
- **Custom Type Extension (`x-rust-type`)**: Added support for the `x-rust-type` OpenAPI vendor extension. When present on a schema, the generator creates a Rust type alias instead of generating a full struct or enum. This allows:
  - Reusing existing domain models instead of generating duplicates
  - Integration with types from other crates
  - Clean separation between API models and domain models
  - Works with any schema type (object, enum, oneOf, anyOf, allOf)
- **Custom Attributes Extension (`x-rust-attrs`)**: Added support for the `x-rust-attrs` OpenAPI vendor extension. Allows adding arbitrary Rust attributes to generated types:
  - Custom derives (e.g., additional traits)
  - Serde rules (rename_all, deny_unknown_fields, etc.)
  - Conditional compilation attributes (cfg, cfg_attr)
  - Works with structs, enums, unions, compositions, and type aliases
  - Preserves attribute order
  - Compatible with `x-rust-type` extension

### Fixed
- **Nullable Fields in Referenced Schemas**: Fixed handling of `nullable` flag for fields that use `$ref` to reference other schemas. The nullable flag is now correctly resolved from the target schema.
- **Required Fields in `allOf` Compositions**: Fixed merging of `required` fields in `allOf` compositions. Previously, only the `required` fields from the last schema were considered. Now all `required` fields from all schemas in the composition are properly collected and merged.

## [0.3.1] - 2025-11-04
### Fixed
- **Response generation**: Fixed issue with response generation. [GitHub issue](https://github.com/denislituev/openapi-model-generator/issues/12)

## [0.3.0] - 2025-10-21
### Added
- **Enum Support**: The generator now automatically creates Rust enums from OpenAPI string schemas that include an enum constraint. This provides type-safe representations for fixed sets of values.

### Fixed
- **Nullable Field Handling in allOf**: Resolved a critical issue where nullable fields within allOf compositions were incorrectly generating Option<Option<T>> types. The parser and generator now correctly produce a single Option<T>.
- **oneOf Serialization**: Fixed the generation of oneOf schemas to use #[serde(untagged)] instead of the default tagged serialization. This ensures that the generated Rust enums correctly serialize and deserialize according to the OpenAPI oneOf specification, which does not require a discriminator field.

## [0.2.2] - 2025-09-11

### Fixed
- **Name Normalization**: Fixed parser to consistently normalize schema, model, and variant names into PascalCase
- **Idiomatic Rust Naming**: All generated Rust types now follow proper naming conventions
- **Better Word Separation**: Improved `to_pascal_case` function to handle `-` and `_` as word separators
- **Schema Reference Naming**: Applied PascalCase normalization when resolving `$ref` schema references
- **Union Variant Naming**: Fixed naming of union variants to follow PascalCase conventions

### Changed
- Schema names with dashes or underscores (e.g., `user-name`, `list_roles`) now generate idiomatic Rust identifiers (`UserName`, `ListRoles`)
- Prevents compilation errors from invalid Rust identifiers in generated code

## [0.2.1] - 2025-09-06

### Added
- **Nullable Field Support**: Fields with `nullable: true` now generate `Option<T>` type correctly
- **Smart Option Generation**: Fields become `Option<T>` when either not required OR nullable
- **Enhanced Type Safety**: Better representation of optional vs nullable fields in generated Rust code

## [0.2.0] - 2025-08-26

### Added
- **UUID Format Support**: Fields with `type: string, format: uuid` now generate `Uuid` type instead of `String`
- **DateTime Format Support**: Fields with `type: string, format: date-time` now generate `DateTime<Utc>` type  
- **Schema Composition Support**: Full support for OpenAPI composition patterns:
  - `allOf`: Combines multiple schemas into a single struct with merged fields
  - `oneOf`: Generates tagged union enum for exclusive choice between schemas
  - `anyOf`: Generates tagged union enum for inclusive choice between schemas
- **Improved Code Generation**:
  - Smart naming for union variants (uses schema names for `$ref`, auto-generates for inline schemas)
  - PascalCase naming for request/response models (follows Rust conventions)
  - Proper `$ref` reference resolution in complex schema hierarchies
- **Library API**: Export of main functions (`parse_openapi`, `generate_models`) for use as a library

## [0.1.0] - Initial Release

### Added
- Basic OpenAPI 3.0 specification parsing
- Simple struct generation for OpenAPI schemas
- Support for basic Rust types (String, i64, bool, etc.)
- Serde integration with proper field renaming
- Command-line interface with input/output options
- Support for required and optional fields
- Array type generation with `Vec<T>`
- Basic error handling and validation
