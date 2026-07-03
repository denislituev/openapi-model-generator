/// Integration tests for the full parse → generate pipeline.
///
/// These tests use only the public API:
///   - `openapi_model_generator::parse_openapi`
///   - `openapi_model_generator::generate_models`
///   - `openapi_model_generator::GenerateMode`
use openapi_model_generator::{generate_models, parse_openapi, GenerateMode};
use openapiv3::OpenAPI;
use serde_json::json;

fn parse_spec(value: serde_json::Value) -> OpenAPI {
    serde_json::from_value(value).expect("Failed to deserialize OpenAPI spec")
}

// ─── Basic struct generation ─────────────────────────────────────────────────

#[test]
fn test_pipeline_simple_struct() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "User": {
                    "type": "object",
                    "properties": {
                        "id":    { "type": "string" },
                        "email": { "type": "string" },
                        "age":   { "type": "integer" }
                    },
                    "required": ["id", "email"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub struct User {"), "Expected struct User:\n{code}");
    assert!(code.contains("pub id: String,"), "id must be non-optional:\n{code}");
    assert!(code.contains("pub email: String,"), "email must be non-optional:\n{code}");
    assert!(code.contains("pub age: Option<i64>,"), "age must be optional:\n{code}");
    assert!(code.contains("use serde::{Serialize, Deserialize};"));
}

// ─── Enum generation ─────────────────────────────────────────────────────────

#[test]
fn test_pipeline_enum() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Status": {
                    "type": "string",
                    "enum": ["pending", "active", "inactive"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub enum Status {"), "Expected enum Status:\n{code}");
    assert!(code.contains("Pending"), "Expected Pending variant:\n{code}");
    assert!(code.contains("Active"), "Expected Active variant:\n{code}");
    assert!(code.contains("Inactive"), "Expected Inactive variant:\n{code}");
}

// ─── allOf composition ───────────────────────────────────────────────────────

#[test]
fn test_pipeline_allof_composition() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Base": {
                    "type": "object",
                    "properties": { "id": { "type": "string" } },
                    "required": ["id"]
                },
                "Extended": {
                    "allOf": [
                        { "$ref": "#/components/schemas/Base" },
                        {
                            "type": "object",
                            "properties": { "extra": { "type": "string" } }
                        }
                    ]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub struct Extended {"), "Expected Extended struct:\n{code}");
    assert!(code.contains("pub id: String,"), "id should be non-optional (required in Base):\n{code}");
    assert!(code.contains("pub extra: Option<String>,"), "extra should be optional:\n{code}");
    assert!(code.contains("/// Extended (allOf composition)"));
}

// ─── oneOf union ─────────────────────────────────────────────────────────────

#[test]
fn test_pipeline_oneof_union() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Cat": { "type": "object", "properties": { "meow": { "type": "string" } } },
                "Dog": { "type": "object", "properties": { "bark": { "type": "string" } } },
                "Pet": {
                    "oneOf": [
                        { "$ref": "#/components/schemas/Cat" },
                        { "$ref": "#/components/schemas/Dog" }
                    ]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub enum Pet {"), "Expected Pet enum:\n{code}");
    assert!(code.contains("#[serde(untagged)]"), "oneOf must be untagged:\n{code}");
    assert!(code.contains("/// Pet (oneOf)"), "Expected oneOf doc comment:\n{code}");
}

// ─── anyOf union ─────────────────────────────────────────────────────────────

#[test]
fn test_pipeline_anyof_union() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Alert": {
                    "anyOf": [
                        { "type": "object", "properties": { "email": { "type": "string" } } },
                        { "type": "object", "properties": { "sms":   { "type": "string" } } }
                    ]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub enum Alert {"), "Expected Alert enum:\n{code}");
    assert!(code.contains("/// Alert (anyOf)"), "Expected anyOf doc comment:\n{code}");
}

// ─── Type alias (x-rust-type) ────────────────────────────────────────────────

#[test]
fn test_pipeline_type_alias_from_x_rust_type() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "UserId": {
                    "type": "string",
                    "format": "uuid",
                    "x-rust-type": "uuid::Uuid"
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub type UserId = uuid::Uuid;"), "Expected type alias:\n{code}");
}

// ─── Request and response generation ─────────────────────────────────────────

#[test]
fn test_pipeline_requests_and_responses_with_all_mode() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {
            "/users": {
                "post": {
                    "operationId": "createUser",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "name": { "type": "string" }
                                    },
                                    "required": ["name"]
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Created",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/User" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "User": {
                    "type": "object",
                    "properties": {
                        "id":   { "type": "string" },
                        "name": { "type": "string" }
                    },
                    "required": ["id"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::ALL, false)
        .expect("generate_models failed");

    assert!(code.contains("pub struct CreateUserRequest {"), "Expected request struct:\n{code}");
    assert!(code.contains("pub struct CreateUserResponse201 {"), "Expected response struct:\n{code}");
    assert!(code.contains("pub struct User {"), "Expected User model:\n{code}");
}

#[test]
fn test_pipeline_models_only_mode_omits_request_response() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {
            "/items": {
                "post": {
                    "operationId": "createItem",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/Item" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "OK",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/Item" }
                                }
                            }
                        }
                    }
                }
            }
        },
        "components": {
            "schemas": {
                "Item": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } }
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(!code.contains("CreateItemRequest"), "MODELS mode must not emit request types:\n{code}");
    assert!(!code.contains("CreateItemResponse"), "MODELS mode must not emit response types:\n{code}");
    assert!(code.contains("pub struct Item {"), "Item model should still be generated:\n{code}");
}

// ─── Display flag ────────────────────────────────────────────────────────────

#[test]
fn test_pipeline_display_flag_adds_display_impl() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Role": {
                    "type": "string",
                    "enum": ["admin", "user", "guest"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");

    let code_with_display =
        generate_models(&models, &requests, &responses, GenerateMode::MODELS, true)
            .expect("generate_models failed");
    assert!(
        code_with_display.contains("impl std::fmt::Display for Role"),
        "Display flag should add Display impl:\n{code_with_display}"
    );

    let code_without_display =
        generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
            .expect("generate_models failed");
    assert!(
        !code_without_display.contains("impl std::fmt::Display"),
        "Display flag off should not add Display impl:\n{code_without_display}"
    );
}

// ─── Chrono / UUID imports ────────────────────────────────────────────────────

#[test]
fn test_pipeline_chrono_import_added_for_datetime_field() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Event": {
                    "type": "object",
                    "properties": {
                        "occurred_at": { "type": "string", "format": "date-time" }
                    },
                    "required": ["occurred_at"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("use chrono::{"), "Expected chrono import:\n{code}");
    assert!(code.contains("DateTime"), "Expected DateTime in chrono import:\n{code}");
    assert!(code.contains("pub occurred_at: DateTime<Utc>,"), "Expected DateTime<Utc> field:\n{code}");
}

#[test]
fn test_pipeline_date_import_added_for_date_field() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Schedule": {
                    "type": "object",
                    "properties": {
                        "date": { "type": "string", "format": "date" }
                    },
                    "required": ["date"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("NaiveDate"), "Expected NaiveDate import:\n{code}");
    assert!(code.contains("pub date: NaiveDate,"), "Expected NaiveDate field:\n{code}");
}

#[test]
fn test_pipeline_uuid_import_added_for_uuid_field() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Resource": {
                    "type": "object",
                    "properties": {
                        "id": { "type": "string", "format": "uuid" }
                    },
                    "required": ["id"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("use uuid::Uuid;"), "Expected uuid import:\n{code}");
    assert!(code.contains("pub id: Uuid,"), "Expected Uuid field:\n{code}");
}

// ─── No duplicate models for repeated schemas ─────────────────────────────────

#[test]
fn test_pipeline_no_duplicate_models() {
    // The same schema referenced multiple times must produce a single model.
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Tag": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } }
                },
                "Article": {
                    "type": "object",
                    "properties": {
                        "primary_tag":   { "$ref": "#/components/schemas/Tag" },
                        "secondary_tag": { "$ref": "#/components/schemas/Tag" }
                    }
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    // "pub struct Tag {" must appear exactly once
    let count = code.matches("pub struct Tag {").count();
    assert_eq!(count, 1, "Tag struct should appear exactly once:\n{code}");
}

// ─── Inline nested objects ────────────────────────────────────────────────────

#[test]
fn test_pipeline_inline_nested_object_generates_struct() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Order": {
                    "type": "object",
                    "properties": {
                        "address": {
                            "type": "object",
                            "properties": {
                                "street": { "type": "string" },
                                "city":   { "type": "string" }
                            }
                        }
                    }
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub struct Order {"), "Expected Order struct:\n{code}");
    assert!(code.contains("pub struct Address {"), "Expected inline Address struct:\n{code}");
}

// ─── Custom attributes (x-rust-attrs) ────────────────────────────────────────

#[test]
fn test_pipeline_x_rust_attrs_applied_to_generated_struct() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Config": {
                    "type": "object",
                    "x-rust-attrs": ["#[derive(Hash, Eq, PartialEq)]"],
                    "properties": {
                        "key": { "type": "string" }
                    }
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(
        code.contains("#[derive(Hash, Eq, PartialEq)]"),
        "Custom x-rust-attrs must appear in output:\n{code}"
    );
}

// ─── YAML parsing ────────────────────────────────────────────────────────────

#[test]
fn test_pipeline_parse_from_yaml_string() {
    let yaml = r#"
openapi: "3.0.0"
info:
  title: YAML Test API
  version: "1.0.0"
paths: {}
components:
  schemas:
    Widget:
      type: object
      properties:
        name:
          type: string
        count:
          type: integer
      required:
        - name
"#;

    let openapi: OpenAPI = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    let (models, requests, responses) = parse_openapi(&openapi).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub struct Widget {"), "Expected Widget struct from YAML spec:\n{code}");
    assert!(code.contains("pub name: String,"));
    assert!(code.contains("pub count: Option<i64>,"));
}

// ─── Error: empty spec ───────────────────────────────────────────────────────

#[test]
fn test_pipeline_empty_spec_produces_no_models() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Empty API", "version": "1.0.0" },
        "paths": {}
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    assert!(models.is_empty(), "Empty spec should produce no models");
    assert!(requests.is_empty());
    assert!(responses.is_empty());

    let code = generate_models(&models, &requests, &responses, GenerateMode::ALL, false)
        .expect("generate_models failed");
    assert!(code.contains("use serde::{Serialize, Deserialize};"), "Should still have serde import");
}

// ─── Boolean and number fields ────────────────────────────────────────────────

#[test]
fn test_pipeline_boolean_and_number_fields() {
    let spec = parse_spec(json!({
        "openapi": "3.0.0",
        "info": { "title": "Test API", "version": "1.0.0" },
        "paths": {},
        "components": {
            "schemas": {
                "Settings": {
                    "type": "object",
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "ratio":   { "type": "number" },
                        "count":   { "type": "integer" }
                    },
                    "required": ["enabled", "ratio", "count"]
                }
            }
        }
    }));

    let (models, requests, responses) = parse_openapi(&spec).expect("parse_openapi failed");
    let code = generate_models(&models, &requests, &responses, GenerateMode::MODELS, false)
        .expect("generate_models failed");

    assert!(code.contains("pub enabled: bool,"), "Expected bool field:\n{code}");
    assert!(code.contains("pub ratio: f64,"), "Expected f64 field:\n{code}");
    assert!(code.contains("pub count: i64,"), "Expected i64 field:\n{code}");
}
