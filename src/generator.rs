use crate::{models::Model, Result};

const RUST_RESERVED_KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
    "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while",

    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "try",
    "typeof", "unsized", "virtual", "yield",
];

fn is_reserved_word(string_to_check: &str) -> bool {
    RUST_RESERVED_KEYWORDS.contains(&string_to_check)
}

pub fn generate_rust_code(models: &[Model]) -> Result<String> {
    let mut code = String::new();

    code.push_str("use serde::{Serialize, Deserialize};\n");
    code.push_str("use uuid::Uuid;\n");
    code.push_str("use chrono::{DateTime, NaiveDate, Utc};\n\n");

    for model in models {
        code.push_str(&format!("/// {}\n", model.name));
        code.push_str(&format!("#[derive(Debug, Serialize, Deserialize)]\n"));
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

            let mut lowercased_name = field.name.to_lowercase();
            if is_reserved_word(&lowercased_name) {
                lowercased_name = format!("r#{}", lowercased_name)
            }

            code.push_str(&format!(
                "    #[serde(rename = \"{}\")]\n",
                field.name.to_lowercase()
            ));
            
            if field.is_required {
                code.push_str(&format!(
                    "    pub {}: {},\n",
                    lowercased_name,
                    field_type
                ));
            } else {
                code.push_str(&format!(
                    "    pub {}: Option<{}>,\n",
                    lowercased_name,
                    field_type
                ));
            }
        }

        code.push_str("}\n\n");
    }

    Ok(code)
}

pub fn generate_lib() -> Result<String> {
    let mut code = String::new();
    code.push_str("pub mod models;\n");

    Ok(code)
}