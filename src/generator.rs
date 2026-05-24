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
}
