use std::sync::OnceLock;

use crate::{
    models::{
        CompositionModel, EnumModel, Model, ModelType, RequestModel, ResponseModel, TypeAliasModel,
        UnionModel, UnionType,
    },
    Result,
};

bitflags::bitflags! {
    struct RequiredUses: u8 {
        const UUID = 0b00000001;
        const DATETIME = 0b00000010;
        const DATE = 0b00000100;
    }

    /// Choose what type of structs you want to generate:
    ///  - Models (generated always)
    ///  - Requests (optional)
    ///  - Responses (optional)
    pub struct GenerateMode: u8 {
        /// Models will be always include to output
        const MODELS = 0;
        /// Additional includes request structs to output
        const REQUESTS = 1 << 0;
        /// Additional includes response structs to output
        const RESPONSES = 1 << 1;
        /// Outputs all possible structs: models, request and response structs
        const ALL = Self::REQUESTS.bits() | Self::RESPONSES.bits();
    }
}

impl Default for GenerateMode {
    fn default() -> Self {
        Self::ALL
    }
}

static HDR: OnceLock<String> = OnceLock::new();

fn create_header() -> String {
    HDR.get_or_init(|| {
        format!(
            r#"
//!
//! Generated from an OAS specification by {}(v{})
//!

"#,
            option_env!("CARGO_PKG_NAME").unwrap_or("openapi-model-generator"),
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown")
        )
    })
    .clone()
}

const RUST_RESERVED_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "abstract", "become", "box", "do", "final", "gen", "macro", "override", "priv", "try",
    "typeof", "unsized", "virtual", "yield",
];

const EMPTY_RESPONSE_NAME: &str = "UnknownResponse";
const EMPTY_REQUEST_NAME: &str = "UnknownRequest";

fn is_reserved_word(string_to_check: &str) -> bool {
    RUST_RESERVED_KEYWORDS.contains(&string_to_check.to_lowercase().as_str())
}

fn generate_description_docs(
    description: &Option<String>,
    fallback_str: &str,
    indent: &str,
) -> String {
    let mut output = String::new();
    if let Some(desc) = description {
        for line in desc.lines() {
            output.push_str(&format!("{}/// {}\n", indent, line.trim()));
        }
    } else if !fallback_str.is_empty() {
        output.push_str(&format!("{}/// {}\n", indent, fallback_str));
    }

    output
}

fn to_snake_case(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();

    let mut snake = String::new();

    for (i, c) in cleaned.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i != 0 {
                snake.push('_');
            }
            snake.push(c.to_ascii_lowercase());
        } else {
            snake.push(c);
        }
    }
    snake = snake.replace("__", "_");

    if snake == "self" {
        snake.push('_');
    }

    if snake
        .chars()
        .next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
        snake = format!("_{snake}");
    }

    snake
}

/// Checks if custom attributes contain a derive attribute
fn has_custom_derive(custom_attrs: &Option<Vec<String>>) -> bool {
    if let Some(attrs) = custom_attrs {
        attrs
            .iter()
            .any(|attr| attr.trim().starts_with("#[derive("))
    } else {
        false
    }
}

/// Checks if custom attributes contain a serde attribute
fn has_custom_serde(custom_attrs: &Option<Vec<String>>) -> bool {
    if let Some(attrs) = custom_attrs {
        attrs.iter().any(|attr| attr.trim().starts_with("#[serde("))
    } else {
        false
    }
}

/// Generates a `impl std::fmt::Display` block for a named type, unless custom_attrs
/// already contains a Display derive. Structs use `{:?}` (Debug) as a fallback;
/// for enums and unions the caller supplies the match body via `match_arms`.
fn generate_display_impl(name: &str, custom_attrs: &Option<Vec<String>>, body: &str) -> String {
    // TODO: `.contains("Display")` is a loose heuristic - it correctly catches
    // `derive_more::Display` and `#[display(...)]` format attrs, but could
    // false-positive on unrelated attribute strings containing "Display".
    // The proper fix is a dedicated spec extension (`x-rust-display: false`)
    // that explicitly opts a type out of Display generation, rather than
    // inferring intent from x-rust-attrs content.
    let has_display = custom_attrs
        .as_ref()
        .is_some_and(|attrs| attrs.iter().any(|a| a.contains("Display")));
    if has_display {
        return String::new();
    }
    format!(
        "impl std::fmt::Display for {name} {{\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{\n{body}    }}\n}}\n"
    )
}

/// Generates custom attributes from x-rust-attrs
fn generate_custom_attrs(custom_attrs: &Option<Vec<String>>) -> String {
    if let Some(attrs) = custom_attrs {
        attrs
            .iter()
            .map(|attr| format!("{attr}\n"))
            .collect::<String>()
    } else {
        String::new()
    }
}

pub fn generate_models(
    models: &[ModelType],
    requests: &[RequestModel],
    responses: &[ResponseModel],
    mode: GenerateMode,
    display: bool,
) -> Result<String> {
    // First, generate all model code to determine which imports are needed
    let mut models_code = String::new();
    let mut required_uses = RequiredUses::empty();
    let mut needs_validator = false;

    for model_type in models {
        match model_type {
            ModelType::Struct(model) => {
                models_code.push_str(&generate_model(
                    model,
                    &mut required_uses,
                    &mut needs_validator,
                    display,
                )?);
            }
            ModelType::Union(union) => {
                models_code.push_str(&generate_union(union, display)?);
            }
            ModelType::Composition(comp) => {
                models_code.push_str(&generate_composition(comp, &mut required_uses, display)?);
            }
            ModelType::Enum(enum_model) => {
                models_code.push_str(&generate_enum(enum_model, display)?);
            }
            ModelType::TypeAlias(type_alias) => {
                models_code.push_str(&generate_type_alias(type_alias)?);
            }
        }
    }

    if mode.contains(GenerateMode::REQUESTS) {
        for request in requests {
            models_code.push_str(&generate_request_model(request)?);
        }
    }

    if mode.contains(GenerateMode::RESPONSES) {
        for response in responses {
            models_code.push_str(&generate_response_model(response)?);
        }
    }

    // Determine which imports are actually needed
    let needs_uuid = required_uses.contains(RequiredUses::UUID);
    let needs_datetime = required_uses.contains(RequiredUses::DATETIME);
    let needs_date = required_uses.contains(RequiredUses::DATE);

    // Build final output with only necessary imports
    let mut output = create_header();
    output.push_str("use serde::{Serialize, Deserialize};\n");

    if needs_uuid {
        output.push_str("use uuid::Uuid;\n");
    }

    if needs_validator {
        output.push_str("use validator::Validate;\n");
    }

    if needs_datetime || needs_date {
        output.push_str("use chrono::{");
        let mut chrono_imports = Vec::new();
        if needs_datetime {
            chrono_imports.push("DateTime");
        }
        if needs_date {
            chrono_imports.push("NaiveDate");
        }
        if needs_datetime {
            chrono_imports.push("Utc");
        }
        output.push_str(&chrono_imports.join(", "));
        output.push_str("};\n");
    }

    output.push('\n');
    output.push_str(&models_code);

    Ok(output)
}

/// Generate validator attributes based on validation rules
fn generate_validator_attrs(rules: &crate::models::ValidationRules, field_type: &str) -> String {
    let mut attrs = String::new();

    match field_type {
        "String" | "str" | "Option<String>" | "Option<str>" => {
            let mut length_attrs = Vec::new();
            if let Some(min) = rules.min_length {
                length_attrs.push(format!("min = {}", min));
            }
            if let Some(max) = rules.max_length {
                length_attrs.push(format!("max = {}", max));
            }
            if !length_attrs.is_empty() {
                attrs.push_str(&format!(
                    "    #[validate(length({}))]\n",
                    length_attrs.join(", ")
                ));
            }

            if rules.email {
                attrs.push_str("    #[validate(email)]\n");
            }

            if rules.url {
                attrs.push_str("    #[validate(url)]\n");
            }

            if let Some(pattern) = &rules.pattern {
                attrs.push_str(&format!("    #[regex(pattern = r\"{}\")]\n", pattern));
            }
        }
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
        | "Option<i8>" | "Option<i16>" | "Option<i32>" | "Option<i64>" | "Option<u8>"
        | "Option<u16>" | "Option<u32>" | "Option<u64>" | "Option<f32>" | "Option<f64>" => {
            let mut range_attrs = Vec::new();
            if let Some(min) = rules.minimum {
                range_attrs.push(format!("min = {}", min));
            }
            if let Some(max) = rules.maximum {
                range_attrs.push(format!("max = {}", max));
            }
            if rules.exclusive_minimum || rules.exclusive_maximum {
                range_attrs.push("exclusive = true".to_string());
            }
            if !range_attrs.is_empty() {
                attrs.push_str(&format!(
                    "    #[validate(range({}))]\n",
                    range_attrs.join(", ")
                ));
            }
        }
        _ if field_type.contains("Vec<") => {
            let mut length_attrs = Vec::new();
            if let Some(min) = rules.min_items {
                length_attrs.push(format!("min = {}", min));
            }
            if let Some(max) = rules.max_items {
                length_attrs.push(format!("max = {}", max));
            }
            if !length_attrs.is_empty() {
                attrs.push_str(&format!(
                    "    #[validate(length({}))]\n",
                    length_attrs.join(", ")
                ));
            }
        }
        _ => {}
    }

    attrs
}

fn generate_model(
    model: &Model,
    required_uses: &mut RequiredUses,
    needs_validator: &mut bool,
    display: bool,
) -> Result<String> {
    let mut output = String::new();

    output.push_str(&generate_description_docs(
        &model.description,
        &model.name,
        "",
    ));

    output.push_str(&generate_custom_attrs(&model.custom_attrs));

    // First pass over fields: resolve types and generate field bodies, tracking
    // whether any #[validate(...)] attrs are needed. This lets us emit the correct
    // derive line once without fragile byte-range patching.
    struct FieldOutput {
        body: String,
        needs_validate: bool,
    }
    let mut field_outputs: Vec<FieldOutput> = Vec::with_capacity(model.fields.len());

    for field in &model.fields {
        let field_type = match field.field_type.as_str() {
            "DateTime" | "DateTime<Utc>" => {
                *required_uses |= RequiredUses::DATETIME;
                "DateTime<Utc>"
            }
            "Date" => {
                *required_uses |= RequiredUses::DATE;
                "NaiveDate"
            }
            "Uuid" => {
                *required_uses |= RequiredUses::UUID;
                "Uuid"
            }
            _ => &field.field_type,
        };

        let mut lowercased_name = to_snake_case(field.name.as_str());
        if is_reserved_word(&lowercased_name) {
            lowercased_name = format!("r#{lowercased_name}")
        }

        let is_optional = !field.is_required || field.is_nullable;
        let base_type = if field.is_array_ref {
            format!("Vec<{field_type}>")
        } else {
            field_type.to_string()
        };
        let full_field_type = if is_optional {
            format!("Option<{base_type}>")
        } else {
            base_type
        };

        let mut field_body = String::new();
        field_body.push_str(&generate_description_docs(&field.description, "", "    "));

        if let Some(attrs) = &field.custom_attrs {
            for attr in attrs {
                field_body.push_str(&format!("    {attr}\n"));
            }
        }

        let mut needs_validate = false;
        if let Some(rules) = &field.validation_rules {
            let attrs = generate_validator_attrs(rules, &full_field_type);
            if !attrs.is_empty() {
                needs_validate = true;
                field_body.push_str(&attrs);
            }
        }

        if lowercased_name != field.name {
            field_body.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
        }
        if field.should_flatten() {
            field_body.push_str("    #[serde(flatten)]\n");
        }
        field_body.push_str(&format!("    pub {lowercased_name}: {full_field_type},\n"));

        field_outputs.push(FieldOutput {
            body: field_body,
            needs_validate,
        });
    }

    let any_validate_attrs = field_outputs.iter().any(|f| f.needs_validate);

    if !has_custom_derive(&model.custom_attrs) {
        if any_validate_attrs {
            *needs_validator = true;
            output.push_str("#[derive(Debug, Clone, Serialize, Deserialize, Validate)]\n");
        } else {
            output.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        }
    }

    output.push_str(&format!("pub struct {} {{\n", model.name));
    for fo in field_outputs {
        output.push_str(&fo.body);
    }

    output.push_str("}\n");
    if display {
        output.push_str(&generate_display_impl(
            &model.name,
            &model.custom_attrs,
            "        write!(f, \"{:?}\", self)\n",
        ));
    }
    output.push('\n');
    Ok(output)
}

fn generate_request_model(request: &RequestModel) -> Result<String> {
    let mut output = String::new();

    if request.name.is_empty() || request.name == EMPTY_REQUEST_NAME {
        return Ok(String::new());
    }

    output.push_str(&format!("/// {}\n", request.name));
    output.push_str("#[derive(Debug, Clone, Serialize)]\n");
    output.push_str(&format!("pub struct {} {{\n", request.name));
    output.push_str(&format!("    pub body: {},\n", request.schema));
    output.push_str("}\n");
    Ok(output)
}

fn generate_response_model(response: &ResponseModel) -> Result<String> {
    if response.name.is_empty() || response.name == EMPTY_RESPONSE_NAME {
        return Ok(String::new());
    }

    let type_name = format!("{}{}", response.name, response.status_code);

    let mut output = String::new();

    output.push_str(&generate_description_docs(
        &response.description,
        &type_name,
        "",
    ));

    output.push_str("#[derive(Debug, Clone, Deserialize)]\n");
    output.push_str(&format!("pub struct {type_name} {{\n"));
    output.push_str(&format!("    pub body: {},\n", response.schema));
    output.push_str("}\n");

    Ok(output)
}

fn generate_union(union: &UnionModel, display: bool) -> Result<String> {
    let mut output = String::new();

    output.push_str(&format!(
        "/// {} ({})\n",
        union.name,
        match union.union_type {
            UnionType::OneOf => "oneOf",
            UnionType::AnyOf => "anyOf",
        }
    ));
    output.push_str(&generate_custom_attrs(&union.custom_attrs));

    // Only add default derive if custom_attrs doesn't already contain a derive
    if !has_custom_derive(&union.custom_attrs) {
        output.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    }

    // Only add default serde(untagged) if custom_attrs doesn't already contain a serde attribute
    if !has_custom_serde(&union.custom_attrs) {
        output.push_str("#[serde(untagged)]\n");
    }

    output.push_str(&format!("pub enum {} {{\n", union.name));

    for variant in &union.variants {
        match &variant.primitive_type {
            Some(t) => output.push_str(&format!("    {}({}),\n", variant.name, t)),
            None => output.push_str(&format!("    {}({}),\n", variant.name, variant.name)),
        }
    }

    output.push_str("}\n");

    if display {
        let match_arms = union
            .variants
            .iter()
            .map(|v| {
                format!(
                    "            Self::{}(inner) => write!(f, \"{{}}\", inner),\n",
                    v.name
                )
            })
            .collect::<String>();
        output.push_str(&generate_display_impl(
            &union.name,
            &union.custom_attrs,
            &format!("        match self {{\n{match_arms}        }}\n"),
        ));
    }

    Ok(output)
}

fn generate_composition(
    comp: &CompositionModel,
    required_uses: &mut RequiredUses,
    display: bool,
) -> Result<String> {
    let mut output = String::new();

    output.push_str(&format!("/// {} (allOf composition)\n", comp.name));
    output.push_str(&generate_custom_attrs(&comp.custom_attrs));

    // Only add default derive if custom_attrs doesn't already contain a derive
    if !has_custom_derive(&comp.custom_attrs) {
        output.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    }

    output.push_str(&format!("pub struct {} {{\n", comp.name));

    for field in &comp.all_fields {
        let field_type = match field.field_type.as_str() {
            "String" => "String",
            "f64" => "f64",
            "i64" => "i64",
            "bool" => "bool",
            "DateTime" => {
                *required_uses |= RequiredUses::DATETIME;
                "DateTime<Utc>"
            }
            "Date" => {
                *required_uses |= RequiredUses::DATE;
                "NaiveDate"
            }
            "Uuid" => {
                *required_uses |= RequiredUses::UUID;
                "Uuid"
            }
            _ => &field.field_type,
        };

        let mut lowercased_name = to_snake_case(field.name.as_str());
        if is_reserved_word(&lowercased_name) {
            lowercased_name = format!("r#{lowercased_name}");
        }

        // Only add serde rename if the Rust field name differs from the original field name
        if lowercased_name != field.name {
            output.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
        }

        // Field-level custom attributes (e.g. #[serde(rename = "...")])
        if let Some(attrs) = &field.custom_attrs {
            for attr in attrs {
                output.push_str(&format!("    {attr}\n"));
            }
        }

        // If field references an array, wrap it in Vec<>
        if field.is_array_ref {
            if field.is_required && !field.is_nullable {
                output.push_str(&format!("    pub {lowercased_name}: Vec<{field_type}>,\n",));
            } else {
                output.push_str(&format!(
                    "    pub {lowercased_name}: Option<Vec<{field_type}>>,\n",
                ));
            }
        } else if field.is_required && !field.is_nullable {
            output.push_str(&format!("    pub {lowercased_name}: {field_type},\n",));
        } else {
            output.push_str(&format!(
                "    pub {lowercased_name}: Option<{field_type}>,\n",
            ));
        }
    }

    output.push_str("}\n");
    if display {
        output.push_str(&generate_display_impl(
            &comp.name,
            &comp.custom_attrs,
            "        write!(f, \"{:?}\", self)\n",
        ));
    }
    Ok(output)
}

fn generate_enum(enum_model: &EnumModel, display: bool) -> Result<String> {
    let mut output = String::new();

    output.push_str(&generate_description_docs(
        &enum_model.description,
        &enum_model.name,
        "",
    ));

    output.push_str(&generate_custom_attrs(&enum_model.custom_attrs));

    // Only add default derive if custom_attrs doesn't already contain a derive
    if !has_custom_derive(&enum_model.custom_attrs) {
        output.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    }

    output.push_str(&format!("pub enum {} {{\n", enum_model.name));

    // Collect (rust_name, display_value) pairs for the Display impl below.
    let mut variant_display: Vec<(String, String)> = Vec::new();

    for (i, variant) in enum_model.variants.iter().enumerate() {
        let original = variant.clone();

        let mut rust_name = crate::parser::to_pascal_case(variant);

        let serde_rename = if is_reserved_word(&rust_name) {
            rust_name.push_str("Value");
            Some(original.clone())
        } else if rust_name != original {
            Some(original.clone())
        } else {
            None
        };

        let display_value = serde_rename
            .as_deref()
            .unwrap_or(&original)
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        variant_display.push((rust_name.clone(), display_value));

        if let Some(rename) = serde_rename {
            output.push_str(&format!("    #[serde(rename = \"{rename}\")]\n"));
        }

        if i + 1 == enum_model.variants.len() {
            output.push_str(&format!("    {rust_name}\n"));
        } else {
            output.push_str(&format!("    {rust_name},\n"));
        }
    }

    output.push_str("}\n");

    if display {
        let match_arms = variant_display
            .iter()
            .map(|(rust_name, display_value)| {
                format!("            Self::{rust_name} => write!(f, \"{display_value}\"),\n")
            })
            .collect::<String>();
        output.push_str(&generate_display_impl(
            &enum_model.name,
            &enum_model.custom_attrs,
            &format!("        match self {{\n{match_arms}        }}\n"),
        ));
    }

    Ok(output)
}

fn generate_type_alias(type_alias: &TypeAliasModel) -> Result<String> {
    let mut output = String::new();

    output.push_str(&generate_description_docs(
        &type_alias.description,
        &type_alias.name,
        "",
    ));

    output.push_str(&generate_custom_attrs(&type_alias.custom_attrs));
    output.push_str(&format!(
        "pub type {} = {};\n\n",
        type_alias.name, type_alias.target_type
    ));

    Ok(output)
}

pub fn generate_rust_code(models: &[Model]) -> Result<String> {
    let mut code = create_header();

    code.push_str("use serde::{Serialize, Deserialize};\n");
    code.push_str("use uuid::Uuid;\n");
    code.push_str("use chrono::{DateTime, NaiveDate, Utc};\n\n");

    for model in models {
        code.push_str(&format!("/// {}\n", model.name));
        code.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        code.push_str(&format!("pub struct {} {{\n", model.name));

        for field in &model.fields {
            let field_type = match field.field_type.as_str() {
                "String" => "String",
                "f64" => "f64",
                "i64" => "i64",
                "bool" => "bool",
                "DateTime" => "DateTime<Utc>",
                "Date" => "NaiveDate",
                "Uuid" => "Uuid",
                _ => &field.field_type,
            };

            let mut lowercased_name = to_snake_case(field.name.as_str());
            if is_reserved_word(&lowercased_name) {
                lowercased_name = format!("r#{lowercased_name}")
            }

            // Only add serde rename if the Rust field name differs from the original field name
            if lowercased_name != field.name {
                code.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
            }

            if field.is_required {
                code.push_str(&format!("    pub {lowercased_name}: {field_type},\n",));
            } else {
                code.push_str(&format!(
                    "    pub {lowercased_name}: Option<{field_type}>,\n",
                ));
            }
        }

        code.push_str("}\n\n");
    }

    Ok(code)
}

pub fn generate_lib() -> Result<String> {
    let mut code = create_header();
    code.push_str("pub mod models;\n");

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::EnumModel;

    fn make_enum(variants: Vec<&str>) -> EnumModel {
        EnumModel {
            name: "TestEnum".to_string(),
            description: None,
            variants: variants.into_iter().map(String::from).collect(),
            custom_attrs: None,
        }
    }

    #[test]
    fn test_enum_display_escapes_quotes_and_backslashes() {
        // Enum values containing " or \ must be escaped in the generated write! string literal.
        let model = make_enum(vec!["normal", r#"with"quote"#, r"with\backslash"]);
        let output = generate_enum(&model, true).expect("generate_enum failed");

        assert!(
            output.contains(r#"write!(f, "with\"quote")"#),
            "double quote should be escaped in Display impl:\n{output}"
        );
        assert!(
            output.contains(r#"write!(f, "with\\backslash")"#),
            "backslash should be escaped in Display impl:\n{output}"
        );
        assert!(
            output.contains(r#"write!(f, "normal")"#),
            "plain value should be unmodified:\n{output}"
        );
    }

    #[test]
    fn test_enum_no_display_when_flag_off() {
        let model = make_enum(vec!["foo", "bar"]);
        let output = generate_enum(&model, false).expect("generate_enum failed");
        assert!(
            !output.contains("impl std::fmt::Display"),
            "Display impl should not be generated when display=false:\n{output}"
        );
    }

    #[test]
    fn test_enum_no_display_when_custom_attrs_has_display() {
        let mut model = make_enum(vec!["foo"]);
        model.custom_attrs = Some(vec![
            "#[derive(derive_more::Display, Debug, Clone)]".to_string()
        ]);
        let output = generate_enum(&model, true).expect("generate_enum failed");
        assert!(
            !output.contains("impl std::fmt::Display"),
            "Display impl should be skipped when x-rust-attrs already has Display:\n{output}"
        );
    }

    // ─── to_snake_case ───────────────────────────────────────────────────────

    #[test]
    fn test_to_snake_case_camel_case() {
        assert_eq!(to_snake_case("camelCase"), "camel_case");
        assert_eq!(to_snake_case("myFieldName"), "my_field_name");
    }

    #[test]
    fn test_to_snake_case_pascal_case() {
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
        assert_eq!(to_snake_case("MyModel"), "my_model");
    }

    #[test]
    fn test_to_snake_case_already_snake() {
        assert_eq!(to_snake_case("snake_case"), "snake_case");
    }

    #[test]
    fn test_to_snake_case_self_reserved() {
        // "Self" -> "self" which is a keyword → appended with '_'
        assert_eq!(to_snake_case("Self"), "self_");
    }

    #[test]
    fn test_to_snake_case_digit_start() {
        // A leading digit must be prefixed with '_'
        assert_eq!(to_snake_case("123field"), "_123field");
    }

    #[test]
    fn test_to_snake_case_special_chars_become_underscore() {
        assert_eq!(to_snake_case("field-name"), "field_name");
        assert_eq!(to_snake_case("field.name"), "field_name");
    }

    #[test]
    fn test_to_snake_case_collapses_double_underscore() {
        // Special chars produce underscores; consecutive underscores are collapsed
        assert_eq!(to_snake_case("field__name"), "field_name");
    }

    // ─── is_reserved_word ────────────────────────────────────────────────────

    #[test]
    fn test_is_reserved_word_fn_keyword() {
        assert!(is_reserved_word("fn"));
        assert!(is_reserved_word("type"));
        assert!(is_reserved_word("struct"));
        assert!(is_reserved_word("while"));
        // raw identifier prefix "r#" is not stripped, so "r#fn" does NOT match
        assert!(!is_reserved_word("r#fn"));
    }

    #[test]
    fn test_is_reserved_word_not_keyword() {
        assert!(!is_reserved_word("name"));
        assert!(!is_reserved_word("id"));
        assert!(!is_reserved_word("value"));
    }

    // ─── has_custom_derive / has_custom_serde ────────────────────────────────

    #[test]
    fn test_has_custom_derive_true() {
        let attrs = Some(vec!["#[derive(Hash)]".to_string()]);
        assert!(has_custom_derive(&attrs));
    }

    #[test]
    fn test_has_custom_derive_false_other_attr() {
        let attrs = Some(vec!["#[serde(rename_all = \"camelCase\")]".to_string()]);
        assert!(!has_custom_derive(&attrs));
    }

    #[test]
    fn test_has_custom_derive_none() {
        assert!(!has_custom_derive(&None));
    }

    #[test]
    fn test_has_custom_serde_true() {
        let attrs = Some(vec!["#[serde(rename_all = \"camelCase\")]".to_string()]);
        assert!(has_custom_serde(&attrs));
    }

    #[test]
    fn test_has_custom_serde_false() {
        let attrs = Some(vec!["#[derive(Hash)]".to_string()]);
        assert!(!has_custom_serde(&attrs));
    }

    // ─── generate_custom_attrs ───────────────────────────────────────────────

    #[test]
    fn test_generate_custom_attrs_some() {
        let attrs = Some(vec!["#[derive(Hash)]".to_string(), "#[serde(rename_all = \"camelCase\")]".to_string()]);
        let out = generate_custom_attrs(&attrs);
        assert!(out.contains("#[derive(Hash)]\n"));
        assert!(out.contains("#[serde(rename_all = \"camelCase\")]\n"));
    }

    #[test]
    fn test_generate_custom_attrs_none() {
        assert_eq!(generate_custom_attrs(&None), "");
    }

    // ─── generate_description_docs ───────────────────────────────────────────

    #[test]
    fn test_generate_description_docs_with_description() {
        let desc = Some("A description.\nSecond line.".to_string());
        let out = generate_description_docs(&desc, "Fallback", "");
        assert!(out.contains("/// A description."));
        assert!(out.contains("/// Second line."));
        assert!(!out.contains("Fallback"));
    }

    #[test]
    fn test_generate_description_docs_uses_fallback() {
        let out = generate_description_docs(&None, "FallbackName", "");
        assert!(out.contains("/// FallbackName"));
    }

    #[test]
    fn test_generate_description_docs_empty_fallback_produces_empty() {
        let out = generate_description_docs(&None, "", "");
        assert!(out.is_empty());
    }

    #[test]
    fn test_generate_description_docs_with_indent() {
        let desc = Some("My field".to_string());
        let out = generate_description_docs(&desc, "", "    ");
        assert!(out.starts_with("    /// My field"));
    }

    // ─── generate_display_impl ───────────────────────────────────────────────

    #[test]
    fn test_generate_display_impl_skipped_when_display_in_custom_attrs() {
        let attrs = Some(vec!["#[derive(Display, Debug)]".to_string()]);
        let out = generate_display_impl("MyType", &attrs, "        write!(f, \"{:?}\", self)\n");
        assert!(out.is_empty(), "Should be empty when Display attr is present");
    }

    #[test]
    fn test_generate_display_impl_generated_when_no_custom_attrs() {
        let out = generate_display_impl("MyType", &None, "        write!(f, \"{:?}\", self)\n");
        assert!(out.contains("impl std::fmt::Display for MyType"));
        assert!(out.contains("write!(f, \"{:?}\", self)"));
    }

    // ─── generate_validator_attrs ────────────────────────────────────────────

    #[test]
    fn test_generate_validator_attrs_string_min_max_length() {
        use crate::models::ValidationRules;
        let rules = ValidationRules {
            min_length: Some(1),
            max_length: Some(100),
            ..Default::default()
        };
        let out = generate_validator_attrs(&rules, "String");
        assert!(out.contains("#[validate(length("));
        assert!(out.contains("min = 1"));
        assert!(out.contains("max = 100"));
    }

    #[test]
    fn test_generate_validator_attrs_string_email() {
        use crate::models::ValidationRules;
        let rules = ValidationRules {
            email: true,
            ..Default::default()
        };
        let out = generate_validator_attrs(&rules, "String");
        assert!(out.contains("#[validate(email)]\n"));
    }

    #[test]
    fn test_generate_validator_attrs_string_url() {
        use crate::models::ValidationRules;
        let rules = ValidationRules {
            url: true,
            ..Default::default()
        };
        let out = generate_validator_attrs(&rules, "Option<String>");
        assert!(out.contains("#[validate(url)]\n"));
    }

    #[test]
    fn test_generate_validator_attrs_string_pattern() {
        use crate::models::ValidationRules;
        let rules = ValidationRules {
            pattern: Some(r"^\d+$".to_string()),
            ..Default::default()
        };
        let out = generate_validator_attrs(&rules, "String");
        assert!(out.contains("#[regex(pattern = r\""));
    }

    #[test]
    fn test_generate_validator_attrs_number_range() {
        use crate::models::ValidationRules;
        let rules = ValidationRules {
            minimum: Some(0.0),
            maximum: Some(255.0),
            ..Default::default()
        };
        let out = generate_validator_attrs(&rules, "i64");
        assert!(out.contains("#[validate(range("));
        assert!(out.contains("min = 0"));
        assert!(out.contains("max = 255"));
    }

    #[test]
    fn test_generate_validator_attrs_array_items() {
        use crate::models::ValidationRules;
        let rules = ValidationRules {
            min_items: Some(1),
            max_items: Some(10),
            ..Default::default()
        };
        let out = generate_validator_attrs(&rules, "Vec<String>");
        assert!(out.contains("#[validate(length("));
        assert!(out.contains("min = 1"));
        assert!(out.contains("max = 10"));
    }

    #[test]
    fn test_generate_validator_attrs_unknown_type_returns_empty() {
        use crate::models::ValidationRules;
        let rules = ValidationRules {
            minimum: Some(1.0),
            ..Default::default()
        };
        // Custom type that doesn't match any arm → no output
        let out = generate_validator_attrs(&rules, "MyCustomType");
        assert!(out.is_empty());
    }

    // ─── generate_model ──────────────────────────────────────────────────────

    fn make_field(name: &str, field_type: &str, is_required: bool) -> crate::models::Field {
        crate::models::Field {
            name: name.to_string(),
            field_type: field_type.to_string(),
            format: "string".to_string(),
            is_required,
            is_nullable: false,
            is_array_ref: false,
            description: None,
            custom_attrs: None,
            validation_rules: None,
        }
    }

    fn make_model(name: &str, fields: Vec<crate::models::Field>) -> crate::models::Model {
        crate::models::Model {
            name: name.to_string(),
            fields,
            custom_attrs: None,
            description: None,
        }
    }

    #[test]
    fn test_generate_model_basic_struct() {
        let model = make_model("User", vec![
            make_field("id", "String", true),
            make_field("age", "i64", false),
        ]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(out.contains("pub struct User {"));
        assert!(out.contains("pub id: String,"));
        assert!(out.contains("pub age: Option<i64>,"));
        assert!(out.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
        assert!(!out.contains("impl std::fmt::Display"));
    }

    #[test]
    fn test_generate_model_with_display() {
        let model = make_model("Product", vec![make_field("name", "String", true)]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, true).expect("generate_model failed");
        assert!(out.contains("impl std::fmt::Display for Product"));
        assert!(out.contains("write!(f, \"{:?}\", self)"));
    }

    #[test]
    fn test_generate_model_datetime_field_sets_import_flag() {
        let model = make_model("Event", vec![make_field("created_at", "DateTime<Utc>", true)]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let _ = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(ru.contains(RequiredUses::DATETIME), "DATETIME flag should be set");
    }

    #[test]
    fn test_generate_model_date_field_sets_import_flag() {
        let model = make_model("Record", vec![make_field("birth_date", "Date", true)]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let _ = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(ru.contains(RequiredUses::DATE), "DATE flag should be set");
    }

    #[test]
    fn test_generate_model_uuid_field_sets_import_flag() {
        let model = make_model("Entity", vec![make_field("id", "Uuid", true)]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(ru.contains(RequiredUses::UUID), "UUID flag should be set");
        assert!(out.contains("pub id: Uuid,"));
    }

    #[test]
    fn test_generate_model_reserved_field_name_gets_raw_prefix() {
        // "type" is a reserved word → r#type
        let model = make_model("Signal", vec![make_field("type", "String", true)]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(out.contains("r#type"));
        assert!(out.contains("#[serde(rename = \"type\")]"));
    }

    #[test]
    fn test_generate_model_serde_rename_when_name_differs() {
        // camelCase field name gets snake_case rename
        let model = make_model("Obj", vec![make_field("myFieldName", "String", true)]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(out.contains("pub my_field_name: String,"));
        assert!(out.contains("#[serde(rename = \"myFieldName\")]"));
    }

    #[test]
    fn test_generate_model_flatten_additional_properties() {
        let mut field = make_field("additional_properties", "std::collections::HashMap<String, serde_json::Value>", false);
        field.is_required = true;
        let model = make_model("Extensible", vec![field]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(out.contains("#[serde(flatten)]"));
    }

    #[test]
    fn test_generate_model_nullable_field_becomes_option() {
        let mut field = make_field("label", "String", true);
        field.is_nullable = true;
        let model = make_model("Widget", vec![field]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(out.contains("pub label: Option<String>,"));
    }

    #[test]
    fn test_generate_model_array_ref_field() {
        let mut field = make_field("items", "String", true);
        field.is_array_ref = true;
        let model = make_model("Collection", vec![field]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        assert!(out.contains("pub items: Vec<String>,"));
    }

    #[test]
    fn test_generate_model_custom_derive_not_doubled() {
        let mut model = make_model("Custom", vec![make_field("x", "i64", true)]);
        model.custom_attrs = Some(vec!["#[derive(Hash, Eq, Debug, Clone, Serialize, Deserialize)]".to_string()]);
        let mut ru = RequiredUses::empty();
        let mut nv = false;
        let out = generate_model(&model, &mut ru, &mut nv, false).expect("generate_model failed");
        // Default derive must NOT be added when custom derive is already present
        let derive_count = out.matches("#[derive(").count();
        assert_eq!(derive_count, 1, "Only one derive block expected:\n{out}");
    }

    // ─── generate_composition ────────────────────────────────────────────────

    #[test]
    fn test_generate_composition_basic() {
        use crate::models::CompositionModel;
        let comp = CompositionModel {
            name: "PersonComposed".to_string(),
            all_fields: vec![
                make_field("name", "String", true),
                make_field("age", "i64", false),
            ],
            custom_attrs: None,
        };
        let mut ru = RequiredUses::empty();
        let out = generate_composition(&comp, &mut ru, false).expect("generate_composition failed");
        assert!(out.contains("pub struct PersonComposed {"));
        assert!(out.contains("pub name: String,"));
        assert!(out.contains("pub age: Option<i64>,"));
        assert!(out.contains("/// PersonComposed (allOf composition)"));
    }

    #[test]
    fn test_generate_composition_with_display() {
        use crate::models::CompositionModel;
        let comp = CompositionModel {
            name: "Base".to_string(),
            all_fields: vec![make_field("id", "String", true)],
            custom_attrs: None,
        };
        let mut ru = RequiredUses::empty();
        let out = generate_composition(&comp, &mut ru, true).expect("generate_composition failed");
        assert!(out.contains("impl std::fmt::Display for Base"));
    }

    // ─── generate_union ──────────────────────────────────────────────────────

    #[test]
    fn test_generate_union_oneof() {
        use crate::models::{UnionModel, UnionType, UnionVariant};
        let union = UnionModel {
            name: "MyUnion".to_string(),
            variants: vec![
                UnionVariant { name: "VariantA".to_string(), fields: vec![], primitive_type: None },
                UnionVariant { name: "VariantB".to_string(), fields: vec![], primitive_type: None },
            ],
            union_type: UnionType::OneOf,
            custom_attrs: None,
        };
        let out = generate_union(&union, false).expect("generate_union failed");
        assert!(out.contains("/// MyUnion (oneOf)"));
        assert!(out.contains("pub enum MyUnion {"));
        assert!(out.contains("VariantA(VariantA),"));
        assert!(out.contains("VariantB(VariantB),"));
        assert!(out.contains("#[serde(untagged)]"));
    }

    #[test]
    fn test_generate_union_anyof() {
        use crate::models::{UnionModel, UnionType, UnionVariant};
        let union = UnionModel {
            name: "AnyOfUnion".to_string(),
            variants: vec![
                UnionVariant { name: "Opt1".to_string(), fields: vec![], primitive_type: None },
            ],
            union_type: UnionType::AnyOf,
            custom_attrs: None,
        };
        let out = generate_union(&union, false).expect("generate_union failed");
        assert!(out.contains("/// AnyOfUnion (anyOf)"));
    }

    #[test]
    fn test_generate_union_with_display() {
        use crate::models::{UnionModel, UnionType, UnionVariant};
        let union = UnionModel {
            name: "PaymentMethod".to_string(),
            variants: vec![
                UnionVariant { name: "Card".to_string(), fields: vec![], primitive_type: None },
                UnionVariant { name: "Cash".to_string(), fields: vec![], primitive_type: None },
            ],
            union_type: UnionType::OneOf,
            custom_attrs: None,
        };
        let out = generate_union(&union, true).expect("generate_union failed");
        assert!(out.contains("impl std::fmt::Display for PaymentMethod"));
        assert!(out.contains("Self::Card(inner) => write!(f, \"{}\", inner)"));
    }

    #[test]
    fn test_generate_union_with_primitive_type() {
        use crate::models::{UnionModel, UnionType, UnionVariant};
        let union = UnionModel {
            name: "StringOrInt".to_string(),
            variants: vec![
                UnionVariant { name: "StringVariant".to_string(), fields: vec![], primitive_type: Some("String".to_string()) },
                UnionVariant { name: "IntVariant".to_string(), fields: vec![], primitive_type: Some("i64".to_string()) },
            ],
            union_type: UnionType::OneOf,
            custom_attrs: None,
        };
        let out = generate_union(&union, false).expect("generate_union failed");
        assert!(out.contains("StringVariant(String),"));
        assert!(out.contains("IntVariant(i64),"));
    }

    // ─── generate_type_alias ─────────────────────────────────────────────────

    #[test]
    fn test_generate_type_alias_basic() {
        use crate::models::TypeAliasModel;
        let alias = TypeAliasModel {
            name: "UserId".to_string(),
            target_type: "uuid::Uuid".to_string(),
            description: None,
            custom_attrs: None,
        };
        let out = generate_type_alias(&alias).expect("generate_type_alias failed");
        assert!(out.contains("pub type UserId = uuid::Uuid;"));
    }

    #[test]
    fn test_generate_type_alias_with_description() {
        use crate::models::TypeAliasModel;
        let alias = TypeAliasModel {
            name: "Tags".to_string(),
            target_type: "Vec<String>".to_string(),
            description: Some("A list of tags".to_string()),
            custom_attrs: None,
        };
        let out = generate_type_alias(&alias).expect("generate_type_alias failed");
        assert!(out.contains("/// A list of tags"));
        assert!(out.contains("pub type Tags = Vec<String>;"));
    }

    // ─── generate_request_model / generate_response_model ───────────────────

    #[test]
    fn test_generate_request_model_basic() {
        use crate::models::RequestModel;
        let req = RequestModel {
            name: "CreateUserRequest".to_string(),
            content_type: "application/json".to_string(),
            schema: "CreateUserRequestBody".to_string(),
            is_required: true,
        };
        let out = generate_request_model(&req).expect("generate_request_model failed");
        assert!(out.contains("pub struct CreateUserRequest {"));
        assert!(out.contains("pub body: CreateUserRequestBody,"));
    }

    #[test]
    fn test_generate_request_model_empty_name_skipped() {
        use crate::models::RequestModel;
        let req = RequestModel {
            name: "".to_string(),
            content_type: "application/json".to_string(),
            schema: "SomeSchema".to_string(),
            is_required: false,
        };
        let out = generate_request_model(&req).expect("generate_request_model failed");
        assert!(out.is_empty(), "Empty-named request should produce no code");
    }

    #[test]
    fn test_generate_request_model_unknown_name_skipped() {
        use crate::models::RequestModel;
        let req = RequestModel {
            name: EMPTY_REQUEST_NAME.to_string(),
            content_type: "application/json".to_string(),
            schema: "Schema".to_string(),
            is_required: false,
        };
        let out = generate_request_model(&req).expect("generate_request_model failed");
        assert!(out.is_empty());
    }

    #[test]
    fn test_generate_response_model_basic() {
        use crate::models::ResponseModel;
        let resp = ResponseModel {
            name: "GetUserResponse".to_string(),
            status_code: "200".to_string(),
            content_type: "application/json".to_string(),
            schema: "User".to_string(),
            description: Some("Successful response".to_string()),
        };
        let out = generate_response_model(&resp).expect("generate_response_model failed");
        assert!(out.contains("pub struct GetUserResponse200 {"));
        assert!(out.contains("pub body: User,"));
        assert!(out.contains("/// Successful response"));
    }

    #[test]
    fn test_generate_response_model_unknown_name_skipped() {
        use crate::models::ResponseModel;
        let resp = ResponseModel {
            name: EMPTY_RESPONSE_NAME.to_string(),
            status_code: "200".to_string(),
            content_type: "application/json".to_string(),
            schema: "User".to_string(),
            description: None,
        };
        let out = generate_response_model(&resp).expect("generate_response_model failed");
        assert!(out.is_empty());
    }

    // ─── generate_models (public API) ────────────────────────────────────────

    #[test]
    fn test_generate_models_always_includes_serde_import() {
        let out = generate_models(&[], &[], &[], GenerateMode::MODELS, false)
            .expect("generate_models failed");
        assert!(out.contains("use serde::{Serialize, Deserialize};"));
    }

    #[test]
    fn test_generate_models_models_only_mode_skips_requests_and_responses() {
        use crate::models::{RequestModel, ResponseModel};
        let requests = vec![RequestModel {
            name: "CreateFooRequest".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            is_required: true,
        }];
        let responses = vec![ResponseModel {
            name: "GetFooResponse".to_string(),
            status_code: "200".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            description: None,
        }];
        let out = generate_models(&[], &requests, &responses, GenerateMode::MODELS, false)
            .expect("generate_models failed");
        assert!(!out.contains("CreateFooRequest"), "MODELS mode must not emit request types");
        assert!(!out.contains("GetFooResponse200"), "MODELS mode must not emit response types");
    }

    #[test]
    fn test_generate_models_all_mode_includes_requests_and_responses() {
        use crate::models::{RequestModel, ResponseModel};
        let requests = vec![RequestModel {
            name: "CreateFooRequest".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            is_required: true,
        }];
        let responses = vec![ResponseModel {
            name: "GetFooResponse".to_string(),
            status_code: "200".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            description: None,
        }];
        let out = generate_models(&[], &requests, &responses, GenerateMode::ALL, false)
            .expect("generate_models failed");
        assert!(out.contains("CreateFooRequest"));
        assert!(out.contains("GetFooResponse200"));
    }

    #[test]
    fn test_generate_models_requests_mode_includes_only_requests() {
        use crate::models::{RequestModel, ResponseModel};
        let requests = vec![RequestModel {
            name: "CreateFooRequest".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            is_required: true,
        }];
        let responses = vec![ResponseModel {
            name: "GetFooResponse".to_string(),
            status_code: "200".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            description: None,
        }];
        let out = generate_models(&[], &requests, &responses, GenerateMode::REQUESTS, false)
            .expect("generate_models failed");
        assert!(out.contains("CreateFooRequest"), "REQUESTS mode should emit requests");
        assert!(!out.contains("GetFooResponse200"), "REQUESTS mode should not emit responses");
    }

    #[test]
    fn test_generate_models_responses_mode_includes_only_responses() {
        use crate::models::{RequestModel, ResponseModel};
        let requests = vec![RequestModel {
            name: "CreateFooRequest".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            is_required: true,
        }];
        let responses = vec![ResponseModel {
            name: "GetFooResponse".to_string(),
            status_code: "200".to_string(),
            content_type: "application/json".to_string(),
            schema: "Foo".to_string(),
            description: None,
        }];
        let out = generate_models(&[], &requests, &responses, GenerateMode::RESPONSES, false)
            .expect("generate_models failed");
        assert!(!out.contains("CreateFooRequest"), "RESPONSES mode should not emit requests");
        assert!(out.contains("GetFooResponse200"), "RESPONSES mode should emit responses");
    }

    #[test]
    fn test_generate_models_adds_chrono_import_for_datetime_field() {
        let models = vec![ModelType::Struct(make_model(
            "Event",
            vec![make_field("created_at", "DateTime<Utc>", true)],
        ))];
        let out = generate_models(&models, &[], &[], GenerateMode::MODELS, false)
            .expect("generate_models failed");
        assert!(out.contains("use chrono::{"));
        assert!(out.contains("DateTime"));
        assert!(out.contains("Utc"));
    }

    #[test]
    fn test_generate_models_adds_uuid_import_for_uuid_field() {
        let models = vec![ModelType::Struct(make_model(
            "Entity",
            vec![make_field("id", "Uuid", true)],
        ))];
        let out = generate_models(&models, &[], &[], GenerateMode::MODELS, false)
            .expect("generate_models failed");
        assert!(out.contains("use uuid::Uuid;"));
    }

    #[test]
    fn test_generate_models_adds_validator_import_when_validation_rules_present() {
        use crate::models::ValidationRules;
        let mut field = make_field("name", "String", true);
        field.validation_rules = Some(ValidationRules {
            min_length: Some(1),
            max_length: Some(50),
            ..Default::default()
        });
        let models = vec![ModelType::Struct(make_model("Validated", vec![field]))];
        let out = generate_models(&models, &[], &[], GenerateMode::MODELS, false)
            .expect("generate_models failed");
        assert!(out.contains("use validator::Validate;"));
        assert!(out.contains("Validate"));
    }

    #[test]
    fn test_generate_lib() {
        let out = generate_lib().expect("generate_lib failed");
        assert!(out.contains("pub mod models;"));
    }
}
