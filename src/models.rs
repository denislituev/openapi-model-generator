use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelType {
    Struct(Model),
    Union(UnionModel),             // oneOf/anyOf -> enum
    Composition(CompositionModel), // allOf
    Enum(EnumModel),               // enum values -> enum
    TypeAlias(TypeAliasModel),     // x-rust-type -> type alias
}

impl ModelType {
    pub fn name(&self) -> &str {
        match self {
            ModelType::Struct(m) => &m.name,
            ModelType::Enum(e) => &e.name,
            ModelType::Union(u) => &u.name,
            ModelType::Composition(c) => &c.name,
            ModelType::TypeAlias(t) => &t.name,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub name: String,
    pub fields: Vec<Field>,
    pub custom_attrs: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub field_type: String,
    pub format: String,
    pub is_required: bool,
    pub is_nullable: bool,
    pub is_array_ref: bool,
    pub description: Option<String>,
    /// Field-level Rust attributes from x-rust-attrs (e.g. #[serde(rename = "...")])
    #[serde(default)]
    pub custom_attrs: Option<Vec<String>>,
    /// Validation rules extracted from OpenAPI specification
    #[serde(default)]
    pub validation_rules: Option<ValidationRules>,
}

impl Field {
    /// Returns true if this field should be flattened (for additionalProperties)
    pub fn should_flatten(&self) -> bool {
        self.name == "additional_properties"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnionModel {
    pub name: String,
    pub variants: Vec<UnionVariant>,
    pub union_type: UnionType,
    pub custom_attrs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnionType {
    OneOf,
    AnyOf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnionVariant {
    pub name: String,
    pub fields: Vec<Field>,
    pub primitive_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionModel {
    pub name: String,
    pub all_fields: Vec<Field>,
    pub custom_attrs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestModel {
    pub name: String,
    pub content_type: String,
    pub schema: String,
    pub is_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseModel {
    pub name: String,
    pub status_code: String,
    pub content_type: String,
    pub schema: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumModel {
    pub name: String,
    pub variants: Vec<String>,
    pub description: Option<String>,
    pub custom_attrs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAliasModel {
    pub name: String,
    pub target_type: String,
    pub description: Option<String>,
    pub custom_attrs: Option<Vec<String>>,
}

/// Validation rules extracted from OpenAPI specification
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationRules {
    // String validation
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub email: bool,
    pub url: bool,

    // Number validation (stored as f64 to handle both Integer and Number types)
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub exclusive_minimum: bool,
    pub exclusive_maximum: bool,
    pub multiple_of: Option<f64>,

    // Array validation
    pub min_items: Option<usize>,
    pub max_items: Option<usize>,
    pub unique_items: bool,
}

impl ValidationRules {
    /// Returns true if there are any validation rules defined
    pub fn has_any(&self) -> bool {
        self.min_length.is_some()
            || self.max_length.is_some()
            || self.pattern.is_some()
            || self.email
            || self.url
            || self.minimum.is_some()
            || self.maximum.is_some()
            || self.exclusive_minimum
            || self.exclusive_maximum
            || self.multiple_of.is_some()
            || self.min_items.is_some()
            || self.max_items.is_some()
            || self.unique_items
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── ModelType::name ─────────────────────────────────────────────────────

    #[test]
    fn test_model_type_name_struct() {
        let m = ModelType::Struct(Model {
            name: "MyStruct".to_string(),
            fields: vec![],
            custom_attrs: None,
            description: None,
        });
        assert_eq!(m.name(), "MyStruct");
    }

    #[test]
    fn test_model_type_name_enum() {
        let m = ModelType::Enum(EnumModel {
            name: "MyEnum".to_string(),
            variants: vec![],
            description: None,
            custom_attrs: None,
        });
        assert_eq!(m.name(), "MyEnum");
    }

    #[test]
    fn test_model_type_name_union() {
        let m = ModelType::Union(UnionModel {
            name: "MyUnion".to_string(),
            variants: vec![],
            union_type: UnionType::OneOf,
            custom_attrs: None,
        });
        assert_eq!(m.name(), "MyUnion");
    }

    #[test]
    fn test_model_type_name_composition() {
        let m = ModelType::Composition(CompositionModel {
            name: "MyComposition".to_string(),
            all_fields: vec![],
            custom_attrs: None,
        });
        assert_eq!(m.name(), "MyComposition");
    }

    #[test]
    fn test_model_type_name_type_alias() {
        let m = ModelType::TypeAlias(TypeAliasModel {
            name: "MyAlias".to_string(),
            target_type: "String".to_string(),
            description: None,
            custom_attrs: None,
        });
        assert_eq!(m.name(), "MyAlias");
    }

    // ─── Field::should_flatten ───────────────────────────────────────────────

    #[test]
    fn test_field_should_flatten_true_for_additional_properties() {
        let field = Field {
            name: "additional_properties".to_string(),
            field_type: "std::collections::HashMap<String, serde_json::Value>".to_string(),
            format: "".to_string(),
            is_required: false,
            is_nullable: false,
            is_array_ref: false,
            description: None,
            custom_attrs: None,
            validation_rules: None,
        };
        assert!(field.should_flatten());
    }

    #[test]
    fn test_field_should_flatten_false_for_regular_field() {
        let field = Field {
            name: "user_id".to_string(),
            field_type: "String".to_string(),
            format: "".to_string(),
            is_required: true,
            is_nullable: false,
            is_array_ref: false,
            description: None,
            custom_attrs: None,
            validation_rules: None,
        };
        assert!(!field.should_flatten());
    }

    // ─── ValidationRules::has_any ────────────────────────────────────────────

    #[test]
    fn test_validation_rules_has_any_default_is_false() {
        let rules = ValidationRules::default();
        assert!(!rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_min_length() {
        let rules = ValidationRules { min_length: Some(1), ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_max_length() {
        let rules = ValidationRules { max_length: Some(100), ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_pattern() {
        let rules = ValidationRules { pattern: Some(r"^\d+$".to_string()), ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_email() {
        let rules = ValidationRules { email: true, ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_url() {
        let rules = ValidationRules { url: true, ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_minimum() {
        let rules = ValidationRules { minimum: Some(0.0), ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_exclusive_minimum() {
        let rules = ValidationRules { exclusive_minimum: true, ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_multiple_of() {
        let rules = ValidationRules { multiple_of: Some(5.0), ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_min_items() {
        let rules = ValidationRules { min_items: Some(1), ..Default::default() };
        assert!(rules.has_any());
    }

    #[test]
    fn test_validation_rules_has_any_unique_items() {
        let rules = ValidationRules { unique_items: true, ..Default::default() };
        assert!(rules.has_any());
    }
}
