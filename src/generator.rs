use crate::{models::Model, Result};

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

            code.push_str(&format!(
                "    #[serde(rename = \"{}\")]\n",
                field.name.to_lowercase()
            ));
            
            if field.is_required {
                code.push_str(&format!(
                    "    pub {}: {},\n",
                    field.name.to_lowercase(),
                    field_type
                ));
            } else {
                code.push_str(&format!(
                    "    pub {}: Option<{}>,\n",
                    field.name.to_lowercase(),
                    field_type
                ));
            }
        }

        code.push_str("}\n\n");
    }

    Ok(code)
} 