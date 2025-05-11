use crate::{models::{Model, Field}, Result};
use openapiv3::{OpenAPI, Schema, ReferenceOr};

pub fn parse_openapi(openapi: &OpenAPI) -> Result<Vec<Model>> {
    let mut models = Vec::new();

    if let Some(components) = &openapi.components {
        for (name, schema) in &components.schemas {
            if let ReferenceOr::Item(schema) = schema {
                let fields = extract_fields(schema)?;
                models.push(Model {
                    name: name.clone(),
                    fields,
                });
            }
        }
    }

    Ok(models)
}

fn extract_fields(schema: &Schema) -> Result<Vec<Field>> {
    let mut fields = Vec::new();
    if let openapiv3::SchemaKind::Type(openapiv3::Type::Object(obj)) = &schema.schema_kind {
        for (name, prop_schema) in &obj.properties {
            let field_type = match prop_schema {
                ReferenceOr::Item(inner_schema) => extract_type(inner_schema)?,
                ReferenceOr::Reference { .. } => "serde_json::Value".to_string(),
            };
            let is_required = obj.required.contains(name);
            fields.push(Field {
                name: name.clone(),
                field_type,
                is_required,
            });
        }
    }
    Ok(fields)
}

fn extract_type(schema: &Schema) -> Result<String> {
    match &schema.schema_kind {
        openapiv3::SchemaKind::Type(openapiv3::Type::String(_)) => Ok("String".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Number(_)) => Ok("f64".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Integer(_)) => Ok("i64".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Boolean {}) => Ok("bool".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Object(_)) => Ok("serde_json::Value".to_string()),
        openapiv3::SchemaKind::Type(openapiv3::Type::Array(arr)) => {
            if let Some(items) = &arr.items {
                match items {
                    ReferenceOr::Item(item_schema) => {
                        let inner_type = extract_type(item_schema)?;
                        Ok(format!("Vec<{}>", inner_type))
                    }
                    ReferenceOr::Reference { .. } => Ok("Vec<serde_json::Value>".to_string()),
                }
            } else {
                Ok("Vec<serde_json::Value>".to_string())
            }
        }
        _ => Ok("serde_json::Value".to_string()),
    }
} 