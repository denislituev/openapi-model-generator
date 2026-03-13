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
) -> Result<String> {
    // First, generate all model code to determine which imports are needed
    let mut models_code = String::new();
    let mut required_uses = RequiredUses::empty();

    for model_type in models {
        match model_type {
            ModelType::Struct(model) => {
                models_code.push_str(&generate_model(model, &mut required_uses)?);
            }
            ModelType::Union(union) => {
                models_code.push_str(&generate_union(union)?);
            }
            ModelType::Composition(comp) => {
                models_code.push_str(&generate_composition(comp, &mut required_uses)?);
            }
            ModelType::Enum(enum_model) => {
                models_code.push_str(&generate_enum(enum_model)?);
            }
            ModelType::TypeAlias(type_alias) => {
                models_code.push_str(&generate_type_alias(type_alias)?);
            }
        }
    }

    for request in requests {
        models_code.push_str(&generate_request_model(request)?);
    }

    for response in responses {
        models_code.push_str(&generate_response_model(response)?);
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

fn generate_model(model: &Model, required_uses: &mut RequiredUses) -> Result<String> {
    let mut output = String::new();

    output.push_str(&generate_description_docs(
        &model.description,
        &model.name,
        "",
    ));

    output.push_str(&generate_custom_attrs(&model.custom_attrs));

    // Only add default derive if custom_attrs doesn't already contain a derive directive
    if !has_custom_derive(&model.custom_attrs) {
        output.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    }

    output.push_str(&format!("pub struct {} {{\n", model.name));

    for field in &model.fields {
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
            lowercased_name = format!("r#{lowercased_name}")
        }

        // Add field description if present
        output.push_str(&generate_description_docs(&field.description, "", "    "));

        // Only add serde rename if the Rust field name differs from the original field name
        if lowercased_name != field.name {
            output.push_str(&format!("    #[serde(rename = \"{}\")]\n", field.name));
        }

        if field.should_flatten() {
            output.push_str("    #[serde(flatten)]\n");
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

    output.push_str("}\n\n");
    Ok(output)
}

fn generate_request_model(request: &RequestModel) -> Result<String> {
    let mut output = String::new();
    tracing::info!("Generating request model");
    tracing::info!("{:#?}", request);

    if request.name.is_empty() || request.name == EMPTY_REQUEST_NAME {
        return Ok(String::new());
    }

    output.push_str(&format!("/// {}\n", request.name));
    output.push_str("#[derive(Debug, Clone, Deserialize)]\n");
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

    output.push_str("#[derive(Debug, Clone, Serialize)]\n");
    output.push_str(&format!("pub struct {type_name} {{\n"));
    output.push_str(&format!("    pub body: {},\n", response.schema));
    output.push_str("}\n");

    Ok(output)
}

fn generate_union(union: &UnionModel) -> Result<String> {
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
    Ok(output)
}

fn generate_composition(
    comp: &CompositionModel,
    required_uses: &mut RequiredUses,
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
    Ok(output)
}

fn generate_enum(enum_model: &EnumModel) -> Result<String> {
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

    for (i, variant) in enum_model.variants.iter().enumerate() {
        let original = variant.clone();

        let mut rust_name = crate::parser::to_pascal_case(variant);

        let serde_rename = if is_reserved_word(&rust_name) {
            rust_name.push_str("Value");
            Some(original)
        } else if rust_name != original {
            Some(original)
        } else {
            None
        };

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
