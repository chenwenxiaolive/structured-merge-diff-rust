//! OpenAPI schema types for both v2 (Swagger) and v3.
//!
//! This module defines the schema types used in OpenAPI documents.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// OpenAPI v2 (Swagger) document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAPIv2 {
    /// Swagger version (should be "2.0").
    #[serde(default)]
    pub swagger: String,

    /// API info.
    #[serde(default)]
    pub info: Info,

    /// Schema definitions.
    #[serde(default)]
    pub definitions: BTreeMap<String, SchemaV2>,

    /// API paths.
    #[serde(default)]
    pub paths: BTreeMap<String, serde_json::Value>,
}

/// OpenAPI v3 document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OpenAPIv3 {
    /// OpenAPI version (should be "3.0.x" or "3.1.x").
    #[serde(default)]
    pub openapi: String,

    /// API info.
    #[serde(default)]
    pub info: Info,

    /// Components section containing schemas.
    #[serde(default)]
    pub components: Components,

    /// API paths.
    #[serde(default)]
    pub paths: BTreeMap<String, serde_json::Value>,
}

/// API information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Info {
    /// API title.
    #[serde(default)]
    pub title: String,

    /// API version.
    #[serde(default)]
    pub version: String,

    /// API description.
    #[serde(default)]
    pub description: Option<String>,
}

/// OpenAPI v3 components section.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Components {
    /// Schema definitions.
    #[serde(default)]
    pub schemas: BTreeMap<String, SchemaV3>,
}

/// OpenAPI v2 schema definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaV2 {
    /// Schema type (string, integer, number, boolean, array, object).
    #[serde(rename = "type", default)]
    pub schema_type: Option<String>,

    /// Schema format (int32, int64, float, double, byte, date, date-time, etc.).
    #[serde(default)]
    pub format: Option<String>,

    /// Description of the schema.
    #[serde(default)]
    pub description: Option<String>,

    /// Reference to another schema.
    #[serde(rename = "$ref", default)]
    pub ref_path: Option<String>,

    /// Properties for object types.
    #[serde(default)]
    pub properties: BTreeMap<String, SchemaV2>,

    /// Additional properties for map types.
    #[serde(default)]
    pub additional_properties: Option<Box<AdditionalProperties<SchemaV2>>>,

    /// Items schema for array types.
    #[serde(default)]
    pub items: Option<Box<SchemaV2>>,

    /// Required property names.
    #[serde(default)]
    pub required: Vec<String>,

    /// Default value.
    #[serde(default)]
    pub default: Option<serde_json::Value>,

    /// Enum values.
    #[serde(rename = "enum", default)]
    pub enum_values: Option<Vec<serde_json::Value>>,

    // Kubernetes-specific extensions
    /// x-kubernetes-group-version-kind
    #[serde(rename = "x-kubernetes-group-version-kind", default)]
    pub x_kubernetes_group_version_kind: Option<Vec<GroupVersionKind>>,

    /// x-kubernetes-list-type: atomic, set, or map.
    #[serde(rename = "x-kubernetes-list-type", default)]
    pub x_kubernetes_list_type: Option<String>,

    /// x-kubernetes-list-map-keys: keys for list-type=map.
    #[serde(rename = "x-kubernetes-list-map-keys", default)]
    pub x_kubernetes_list_map_keys: Option<Vec<String>>,

    /// x-kubernetes-map-type: atomic or granular.
    #[serde(rename = "x-kubernetes-map-type", default)]
    pub x_kubernetes_map_type: Option<String>,

    /// x-kubernetes-patch-strategy: merge or replace.
    #[serde(rename = "x-kubernetes-patch-strategy", default)]
    pub x_kubernetes_patch_strategy: Option<String>,

    /// x-kubernetes-patch-merge-key: merge key for strategic merge patch.
    #[serde(rename = "x-kubernetes-patch-merge-key", default)]
    pub x_kubernetes_patch_merge_key: Option<String>,

    /// x-kubernetes-preserve-unknown-fields: preserve unknown fields.
    #[serde(rename = "x-kubernetes-preserve-unknown-fields", default)]
    pub x_kubernetes_preserve_unknown_fields: Option<bool>,

    /// x-kubernetes-int-or-string: field can be int or string.
    #[serde(rename = "x-kubernetes-int-or-string", default)]
    pub x_kubernetes_int_or_string: Option<bool>,

    /// x-kubernetes-embedded-resource: embedded resource.
    #[serde(rename = "x-kubernetes-embedded-resource", default)]
    pub x_kubernetes_embedded_resource: Option<bool>,

    /// x-kubernetes-unions: union discriminators.
    #[serde(rename = "x-kubernetes-unions", default)]
    pub x_kubernetes_unions: Option<Vec<UnionDefinition>>,
}

/// OpenAPI v3 schema definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaV3 {
    /// Schema type (string, integer, number, boolean, array, object).
    #[serde(rename = "type", default)]
    pub schema_type: Option<String>,

    /// Schema format (int32, int64, float, double, byte, date, date-time, etc.).
    #[serde(default)]
    pub format: Option<String>,

    /// Description of the schema.
    #[serde(default)]
    pub description: Option<String>,

    /// Reference to another schema.
    #[serde(rename = "$ref", default)]
    pub ref_path: Option<String>,

    /// Properties for object types.
    #[serde(default)]
    pub properties: BTreeMap<String, SchemaV3>,

    /// Additional properties for map types.
    #[serde(default)]
    pub additional_properties: Option<Box<AdditionalProperties<SchemaV3>>>,

    /// Items schema for array types.
    #[serde(default)]
    pub items: Option<Box<SchemaV3>>,

    /// Required property names.
    #[serde(default)]
    pub required: Vec<String>,

    /// Default value.
    #[serde(default)]
    pub default: Option<serde_json::Value>,

    /// Enum values.
    #[serde(rename = "enum", default)]
    pub enum_values: Option<Vec<serde_json::Value>>,

    /// allOf composition.
    #[serde(rename = "allOf", default)]
    pub all_of: Option<Vec<SchemaV3>>,

    /// anyOf composition.
    #[serde(rename = "anyOf", default)]
    pub any_of: Option<Vec<SchemaV3>>,

    /// oneOf composition.
    #[serde(rename = "oneOf", default)]
    pub one_of: Option<Vec<SchemaV3>>,

    /// not composition.
    #[serde(default)]
    pub not: Option<Box<SchemaV3>>,

    /// Nullable field (v3 specific).
    #[serde(default)]
    pub nullable: Option<bool>,

    // Kubernetes-specific extensions (same as v2)
    #[serde(rename = "x-kubernetes-group-version-kind", default)]
    pub x_kubernetes_group_version_kind: Option<Vec<GroupVersionKind>>,

    #[serde(rename = "x-kubernetes-list-type", default)]
    pub x_kubernetes_list_type: Option<String>,

    #[serde(rename = "x-kubernetes-list-map-keys", default)]
    pub x_kubernetes_list_map_keys: Option<Vec<String>>,

    #[serde(rename = "x-kubernetes-map-type", default)]
    pub x_kubernetes_map_type: Option<String>,

    #[serde(rename = "x-kubernetes-patch-strategy", default)]
    pub x_kubernetes_patch_strategy: Option<String>,

    #[serde(rename = "x-kubernetes-patch-merge-key", default)]
    pub x_kubernetes_patch_merge_key: Option<String>,

    #[serde(rename = "x-kubernetes-preserve-unknown-fields", default)]
    pub x_kubernetes_preserve_unknown_fields: Option<bool>,

    #[serde(rename = "x-kubernetes-int-or-string", default)]
    pub x_kubernetes_int_or_string: Option<bool>,

    #[serde(rename = "x-kubernetes-embedded-resource", default)]
    pub x_kubernetes_embedded_resource: Option<bool>,

    #[serde(rename = "x-kubernetes-unions", default)]
    pub x_kubernetes_unions: Option<Vec<UnionDefinition>>,
}

/// Additional properties can be a boolean or a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AdditionalProperties<S> {
    /// Boolean indicating whether additional properties are allowed.
    Bool(bool),
    /// Schema for additional properties.
    Schema(S),
}

impl<S: Default> Default for AdditionalProperties<S> {
    fn default() -> Self {
        AdditionalProperties::Bool(true)
    }
}

/// Kubernetes GroupVersionKind.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupVersionKind {
    /// API group.
    #[serde(default)]
    pub group: String,

    /// API version.
    #[serde(default)]
    pub version: String,

    /// Resource kind.
    #[serde(default)]
    pub kind: String,
}

/// Kubernetes union definition (x-kubernetes-unions).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnionDefinition {
    /// Discriminator field name.
    #[serde(default)]
    pub discriminator: Option<String>,

    /// Mapping from field names to discriminator values.
    #[serde(rename = "fields-to-discriminateBy", default)]
    pub fields_to_discriminate_by: BTreeMap<String, String>,
}

/// Unified OpenAPI document that can be either v2 or v3.
#[derive(Debug, Clone)]
pub enum OpenAPIDocument {
    V2(OpenAPIv2),
    V3(OpenAPIv3),
}

impl OpenAPIDocument {
    /// Parse an OpenAPI document from JSON.
    pub fn from_json(json: &str) -> Result<Self, OpenAPIParseError> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| OpenAPIParseError::InvalidJson(e.to_string()))?;

        Self::from_value(value)
    }

    /// Parse an OpenAPI document from YAML.
    pub fn from_yaml(yaml: &str) -> Result<Self, OpenAPIParseError> {
        let value: serde_json::Value = serde_yaml::from_str(yaml)
            .map_err(|e| OpenAPIParseError::InvalidYaml(e.to_string()))?;

        Self::from_value(value)
    }

    /// Parse an OpenAPI document from a serde_json::Value.
    pub fn from_value(value: serde_json::Value) -> Result<Self, OpenAPIParseError> {
        // Detect version
        if let Some(swagger) = value.get("swagger").and_then(|v| v.as_str()) {
            if swagger.starts_with("2.") {
                let doc: OpenAPIv2 = serde_json::from_value(value)
                    .map_err(|e| OpenAPIParseError::InvalidSchema(e.to_string()))?;
                return Ok(OpenAPIDocument::V2(doc));
            }
        }

        if let Some(openapi) = value.get("openapi").and_then(|v| v.as_str()) {
            if openapi.starts_with("3.") {
                let doc: OpenAPIv3 = serde_json::from_value(value)
                    .map_err(|e| OpenAPIParseError::InvalidSchema(e.to_string()))?;
                return Ok(OpenAPIDocument::V3(doc));
            }
        }

        Err(OpenAPIParseError::UnknownVersion)
    }

    /// Returns true if this is an OpenAPI v2 document.
    pub fn is_v2(&self) -> bool {
        matches!(self, OpenAPIDocument::V2(_))
    }

    /// Returns true if this is an OpenAPI v3 document.
    pub fn is_v3(&self) -> bool {
        matches!(self, OpenAPIDocument::V3(_))
    }
}

/// Error type for OpenAPI parsing.
#[derive(Debug, Clone)]
pub enum OpenAPIParseError {
    /// Invalid JSON.
    InvalidJson(String),
    /// Invalid YAML.
    InvalidYaml(String),
    /// Invalid schema structure.
    InvalidSchema(String),
    /// Unknown OpenAPI version.
    UnknownVersion,
}

impl std::fmt::Display for OpenAPIParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OpenAPIParseError::InvalidJson(e) => write!(f, "Invalid JSON: {}", e),
            OpenAPIParseError::InvalidYaml(e) => write!(f, "Invalid YAML: {}", e),
            OpenAPIParseError::InvalidSchema(e) => write!(f, "Invalid schema: {}", e),
            OpenAPIParseError::UnknownVersion => write!(f, "Unknown OpenAPI version"),
        }
    }
}

impl std::error::Error for OpenAPIParseError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v2_document() {
        let json = r#"{
            "swagger": "2.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "definitions": {
                "Pet": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "age": {
                            "type": "integer"
                        }
                    }
                }
            }
        }"#;

        let doc = OpenAPIDocument::from_json(json).unwrap();
        assert!(doc.is_v2());

        if let OpenAPIDocument::V2(v2) = doc {
            assert_eq!(v2.swagger, "2.0");
            assert_eq!(v2.info.title, "Test API");
            assert!(v2.definitions.contains_key("Pet"));
        }
    }

    #[test]
    fn test_parse_v3_document() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string"
                            },
                            "age": {
                                "type": "integer"
                            }
                        }
                    }
                }
            }
        }"#;

        let doc = OpenAPIDocument::from_json(json).unwrap();
        assert!(doc.is_v3());

        if let OpenAPIDocument::V3(v3) = doc {
            assert_eq!(v3.openapi, "3.0.0");
            assert_eq!(v3.info.title, "Test API");
            assert!(v3.components.schemas.contains_key("Pet"));
        }
    }

    #[test]
    fn test_parse_kubernetes_extensions() {
        let json = r##"{
            "swagger": "2.0",
            "info": {"title": "K8s", "version": "1.0"},
            "definitions": {
                "ContainerList": {
                    "type": "array",
                    "items": {
                        "$ref": "#/definitions/Container"
                    },
                    "x-kubernetes-list-type": "map",
                    "x-kubernetes-list-map-keys": ["name"]
                },
                "Container": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    }
                }
            }
        }"##;

        let doc = OpenAPIDocument::from_json(json).unwrap();
        if let OpenAPIDocument::V2(v2) = doc {
            let container_list = v2.definitions.get("ContainerList").unwrap();
            assert_eq!(container_list.x_kubernetes_list_type, Some("map".to_string()));
            assert_eq!(container_list.x_kubernetes_list_map_keys, Some(vec!["name".to_string()]));
        }
    }
}
