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
