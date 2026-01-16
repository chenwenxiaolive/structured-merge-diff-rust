//! Converter from OpenAPI schema to SMD (structured-merge-diff) schema.
//!
//! This module converts OpenAPI v2/v3 schemas to the SMD schema format used by
//! structured-merge-diff for server-side apply operations.

use crate::schema::{
    Atom, ElementRelationship, List, Map as SchemaMap, Scalar, Schema, StructField, TypeDef,
    TypeRef, Union, UnionField,
};
use super::schema::{
    AdditionalProperties, OpenAPIDocument, OpenAPIv2, OpenAPIv3, SchemaV2, SchemaV3,
};
use std::collections::BTreeMap;

/// Converter from OpenAPI to SMD schema.
pub struct OpenAPIConverter {
    /// Errors encountered during conversion.
    errors: Vec<ConversionError>,
}

/// Error during OpenAPI to SMD conversion.
#[derive(Debug, Clone)]
pub struct ConversionError {
    /// Path to the schema element.
    pub path: String,
    /// Error message.
    pub message: String,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for ConversionError {}

/// Result of OpenAPI to SMD conversion.
pub struct ConversionResult {
    /// The converted SMD schema.
    pub schema: Schema,
    /// Errors encountered during conversion (non-fatal).
    pub errors: Vec<ConversionError>,
}

impl OpenAPIConverter {
    /// Create a new converter.
    pub fn new() -> Self {
        OpenAPIConverter { errors: Vec::new() }
    }

    /// Convert an OpenAPI document to SMD schema.
    pub fn convert(&mut self, doc: &OpenAPIDocument) -> ConversionResult {
        self.errors.clear();

        let schema = match doc {
            OpenAPIDocument::V2(v2) => self.convert_v2(v2),
            OpenAPIDocument::V3(v3) => self.convert_v3(v3),
        };

        ConversionResult {
            schema,
            errors: std::mem::take(&mut self.errors),
        }
    }

    /// Convert OpenAPI v2 document to SMD schema.
    fn convert_v2(&mut self, doc: &OpenAPIv2) -> Schema {
        let mut types = Vec::new();

        for (name, schema) in &doc.definitions {
            if let Some(type_def) = self.convert_v2_schema(name, schema, &doc.definitions) {
                types.push(type_def);
            }
        }

        Schema::with_types(types)
    }

    /// Convert OpenAPI v3 document to SMD schema.
    fn convert_v3(&mut self, doc: &OpenAPIv3) -> Schema {
        let mut types = Vec::new();

        for (name, schema) in &doc.components.schemas {
            if let Some(type_def) = self.convert_v3_schema(name, schema, &doc.components.schemas) {
                types.push(type_def);
            }
        }

        Schema::with_types(types)
    }

    /// Convert a v2 schema to SMD TypeDef.
    fn convert_v2_schema(
        &mut self,
        name: &str,
        schema: &SchemaV2,
        definitions: &BTreeMap<String, SchemaV2>,
    ) -> Option<TypeDef> {
        let atom = self.schema_v2_to_atom(schema, definitions, name);
        Some(TypeDef {
            name: name.to_string(),
            atom,
        })
    }

    /// Convert a v3 schema to SMD TypeDef.
    fn convert_v3_schema(
        &mut self,
        name: &str,
        schema: &SchemaV3,
        definitions: &BTreeMap<String, SchemaV3>,
    ) -> Option<TypeDef> {
        let atom = self.schema_v3_to_atom(schema, definitions, name);
        Some(TypeDef {
            name: name.to_string(),
            atom,
        })
    }

    /// Convert v2 schema to Atom.
    fn schema_v2_to_atom(
        &mut self,
        schema: &SchemaV2,
        definitions: &BTreeMap<String, SchemaV2>,
        path: &str,
    ) -> Atom {
        // Handle $ref first
        if let Some(ref ref_path) = schema.ref_path {
            return self.ref_to_atom(ref_path);
        }

        // Handle x-kubernetes-int-or-string
        if schema.x_kubernetes_int_or_string == Some(true) {
            return Atom {
                scalar: Some(Scalar::String), // treated as string in SMD
                ..Default::default()
            };
        }

        // Handle x-kubernetes-preserve-unknown-fields or x-kubernetes-embedded-resource
        if schema.x_kubernetes_preserve_unknown_fields == Some(true)
            || schema.x_kubernetes_embedded_resource == Some(true)
        {
            // Return untyped map for raw extension types
            return Atom {
                map: Some(SchemaMap::with_element_type(TypeRef {
                    named_type: Some("__untyped_deduced_".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            };
        }

        let schema_type = schema.schema_type.as_deref().unwrap_or("");

        match schema_type {
            "string" => self.convert_string_type(&schema.format),
            "integer" | "number" => Atom {
                scalar: Some(Scalar::Numeric),
                ..Default::default()
            },
            "boolean" => Atom {
                scalar: Some(Scalar::Boolean),
                ..Default::default()
            },
            "array" => self.convert_v2_array(schema, definitions, path),
            "object" | "" => self.convert_v2_object(schema, definitions, path),
            _ => {
                self.add_error(path, &format!("Unknown type: {}", schema_type));
                Atom {
                    scalar: Some(Scalar::Untyped),
                    ..Default::default()
                }
            }
        }
    }

    /// Convert v3 schema to Atom.
    fn schema_v3_to_atom(
        &mut self,
        schema: &SchemaV3,
        definitions: &BTreeMap<String, SchemaV3>,
        path: &str,
    ) -> Atom {
        // Handle $ref first
        if let Some(ref ref_path) = schema.ref_path {
            return self.ref_to_atom(ref_path);
        }

        // Handle allOf, anyOf, oneOf - flatten them
        if let Some(ref all_of) = schema.all_of {
            if !all_of.is_empty() {
                // For allOf, we merge all schemas
                return self.merge_v3_schemas(all_of, definitions, path);
            }
        }

        if let Some(ref any_of) = schema.any_of {
            if !any_of.is_empty() {
                // For anyOf/oneOf, use the first one as primary
                return self.schema_v3_to_atom(&any_of[0], definitions, path);
            }
        }

        if let Some(ref one_of) = schema.one_of {
            if !one_of.is_empty() {
                return self.schema_v3_to_atom(&one_of[0], definitions, path);
            }
        }

        // Handle x-kubernetes-int-or-string
        if schema.x_kubernetes_int_or_string == Some(true) {
            return Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            };
        }

        // Handle x-kubernetes-preserve-unknown-fields or x-kubernetes-embedded-resource
        if schema.x_kubernetes_preserve_unknown_fields == Some(true)
            || schema.x_kubernetes_embedded_resource == Some(true)
        {
            return Atom {
                map: Some(SchemaMap::with_element_type(TypeRef {
                    named_type: Some("__untyped_deduced_".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            };
        }

        let schema_type = schema.schema_type.as_deref().unwrap_or("");

        match schema_type {
            "string" => self.convert_string_type(&schema.format),
            "integer" | "number" => Atom {
                scalar: Some(Scalar::Numeric),
                ..Default::default()
            },
            "boolean" => Atom {
                scalar: Some(Scalar::Boolean),
                ..Default::default()
            },
            "array" => self.convert_v3_array(schema, definitions, path),
            "object" | "" => self.convert_v3_object(schema, definitions, path),
            _ => {
                self.add_error(path, &format!("Unknown type: {}", schema_type));
                Atom {
                    scalar: Some(Scalar::Untyped),
                    ..Default::default()
                }
            }
        }
    }

    /// Convert string type with format.
    fn convert_string_type(&self, _format: &Option<String>) -> Atom {
        // All string formats map to Scalar::String in SMD
        Atom {
            scalar: Some(Scalar::String),
            ..Default::default()
        }
    }

    /// Convert v2 array schema.
    fn convert_v2_array(
        &mut self,
        schema: &SchemaV2,
        definitions: &BTreeMap<String, SchemaV2>,
        path: &str,
    ) -> Atom {
        let element_type = if let Some(ref items) = schema.items {
            self.v2_schema_to_type_ref(items, definitions, &format!("{}.items", path))
        } else {
            TypeRef {
                named_type: Some("__untyped_atomic_".to_string()),
                ..Default::default()
            }
        };

        let element_relationship = self.get_list_element_relationship_v2(schema);
        let keys = schema.x_kubernetes_list_map_keys.clone().unwrap_or_default();

        Atom {
            list: Some(List {
                element_type,
                element_relationship,
                keys,
            }),
            ..Default::default()
        }
    }

    /// Convert v3 array schema.
    fn convert_v3_array(
        &mut self,
        schema: &SchemaV3,
        definitions: &BTreeMap<String, SchemaV3>,
        path: &str,
    ) -> Atom {
        let element_type = if let Some(ref items) = schema.items {
            self.v3_schema_to_type_ref(items, definitions, &format!("{}.items", path))
        } else {
            TypeRef {
                named_type: Some("__untyped_atomic_".to_string()),
                ..Default::default()
            }
        };

        let element_relationship = self.get_list_element_relationship_v3(schema);
        let keys = schema.x_kubernetes_list_map_keys.clone().unwrap_or_default();

        Atom {
            list: Some(List {
                element_type,
                element_relationship,
                keys,
            }),
            ..Default::default()
        }
    }

    /// Convert v2 object schema.
    fn convert_v2_object(
        &mut self,
        schema: &SchemaV2,
        definitions: &BTreeMap<String, SchemaV2>,
        path: &str,
    ) -> Atom {
        let element_relationship = self.get_map_element_relationship_v2(schema);

        // If it has properties, treat as struct
        if !schema.properties.is_empty() {
            let fields: Vec<StructField> = schema
                .properties
                .iter()
                .map(|(name, prop_schema)| {
                    let field_path = format!("{}.{}", path, name);
                    let field_type = self.v2_schema_to_type_ref(prop_schema, definitions, &field_path);
                    StructField {
                        name: name.clone(),
                        field_type,
                        default: prop_schema.default.clone(),
                    }
                })
                .collect();

            // Handle unions
            let unions = self.convert_unions_v2(schema);

            Atom {
                map: Some(SchemaMap::with_all(
                    fields,
                    self.get_additional_properties_type_v2(schema, definitions, path),
                    element_relationship,
                    unions,
                )),
                ..Default::default()
            }
        } else if let Some(ref additional) = schema.additional_properties {
            // Map type with additionalProperties
            let element_type = match additional.as_ref() {
                AdditionalProperties::Bool(true) => TypeRef {
                    named_type: Some("__untyped_deduced_".to_string()),
                    ..Default::default()
                },
                AdditionalProperties::Bool(false) => TypeRef::default(),
                AdditionalProperties::Schema(s) => {
                    self.v2_schema_to_type_ref(s, definitions, &format!("{}.additionalProperties", path))
                }
            };

            Atom {
                map: Some(SchemaMap::with_element_type_and_relationship(
                    element_type,
                    element_relationship,
                )),
                ..Default::default()
            }
        } else {
            // Empty object - treat as untyped map
            Atom {
                map: Some(SchemaMap::with_element_type_and_relationship(
                    TypeRef {
                        named_type: Some("__untyped_deduced_".to_string()),
                        ..Default::default()
                    },
                    element_relationship,
                )),
                ..Default::default()
            }
        }
    }

    /// Convert v3 object schema.
    fn convert_v3_object(
        &mut self,
        schema: &SchemaV3,
        definitions: &BTreeMap<String, SchemaV3>,
        path: &str,
    ) -> Atom {
        let element_relationship = self.get_map_element_relationship_v3(schema);

        // If it has properties, treat as struct
        if !schema.properties.is_empty() {
            let fields: Vec<StructField> = schema
                .properties
                .iter()
                .map(|(name, prop_schema)| {
                    let field_path = format!("{}.{}", path, name);
                    let field_type = self.v3_schema_to_type_ref(prop_schema, definitions, &field_path);
                    StructField {
                        name: name.clone(),
                        field_type,
                        default: prop_schema.default.clone(),
                    }
                })
                .collect();

            // Handle unions
            let unions = self.convert_unions_v3(schema);

            Atom {
                map: Some(SchemaMap::with_all(
                    fields,
                    self.get_additional_properties_type_v3(schema, definitions, path),
                    element_relationship,
                    unions,
                )),
                ..Default::default()
            }
        } else if let Some(ref additional) = schema.additional_properties {
            let element_type = match additional.as_ref() {
                AdditionalProperties::Bool(true) => TypeRef {
                    named_type: Some("__untyped_deduced_".to_string()),
                    ..Default::default()
                },
                AdditionalProperties::Bool(false) => TypeRef::default(),
                AdditionalProperties::Schema(s) => {
                    self.v3_schema_to_type_ref(s, definitions, &format!("{}.additionalProperties", path))
                }
            };

            Atom {
                map: Some(SchemaMap::with_element_type_and_relationship(
                    element_type,
                    element_relationship,
                )),
                ..Default::default()
            }
        } else {
            Atom {
                map: Some(SchemaMap::with_element_type_and_relationship(
                    TypeRef {
                        named_type: Some("__untyped_deduced_".to_string()),
                        ..Default::default()
                    },
                    element_relationship,
                )),
                ..Default::default()
            }
        }
    }

    /// Convert $ref to Atom with namedType.
    fn ref_to_atom(&self, ref_path: &str) -> Atom {
        let type_name = self.extract_type_name_from_ref(ref_path);
        Atom {
            map: Some(SchemaMap::with_element_type(TypeRef {
                named_type: Some(type_name),
                ..Default::default()
            })),
            ..Default::default()
        }
    }

    /// Convert v2 schema to TypeRef.
    fn v2_schema_to_type_ref(
        &mut self,
        schema: &SchemaV2,
        definitions: &BTreeMap<String, SchemaV2>,
        path: &str,
    ) -> TypeRef {
        // Handle $ref
        if let Some(ref ref_path) = schema.ref_path {
            return TypeRef {
                named_type: Some(self.extract_type_name_from_ref(ref_path)),
                ..Default::default()
            };
        }

        // For inline schemas, we create an inline type
        let atom = self.schema_v2_to_atom(schema, definitions, path);
        TypeRef {
            inlined: Box::new(atom),
            ..Default::default()
        }
    }

    /// Convert v3 schema to TypeRef.
    fn v3_schema_to_type_ref(
        &mut self,
        schema: &SchemaV3,
        definitions: &BTreeMap<String, SchemaV3>,
        path: &str,
    ) -> TypeRef {
        // Handle $ref
        if let Some(ref ref_path) = schema.ref_path {
            return TypeRef {
                named_type: Some(self.extract_type_name_from_ref(ref_path)),
                ..Default::default()
            };
        }

        // For inline schemas, we create an inline type
        let atom = self.schema_v3_to_atom(schema, definitions, path);
        TypeRef {
            inlined: Box::new(atom),
            ..Default::default()
        }
    }

    /// Extract type name from $ref path.
    fn extract_type_name_from_ref(&self, ref_path: &str) -> String {
        // Handle v2 style: #/definitions/TypeName
        if let Some(name) = ref_path.strip_prefix("#/definitions/") {
            return name.to_string();
        }
        // Handle v3 style: #/components/schemas/TypeName
        if let Some(name) = ref_path.strip_prefix("#/components/schemas/") {
            return name.to_string();
        }
        // Fallback: use the last component
        ref_path.rsplit('/').next().unwrap_or(ref_path).to_string()
    }

    /// Get list element relationship from v2 schema.
    fn get_list_element_relationship_v2(&self, schema: &SchemaV2) -> ElementRelationship {
        // Check x-kubernetes-list-type
        match schema.x_kubernetes_list_type.as_deref() {
            Some("atomic") => ElementRelationship::Atomic,
            Some("set") => ElementRelationship::Associative,
            Some("map") => ElementRelationship::Associative,
            _ => {
                // Fallback to x-kubernetes-patch-strategy
                match schema.x_kubernetes_patch_strategy.as_deref() {
                    Some("merge") | Some("retainKeys") => ElementRelationship::Associative,
                    _ => ElementRelationship::Atomic, // Default for lists
                }
            }
        }
    }

    /// Get list element relationship from v3 schema.
    fn get_list_element_relationship_v3(&self, schema: &SchemaV3) -> ElementRelationship {
        match schema.x_kubernetes_list_type.as_deref() {
            Some("atomic") => ElementRelationship::Atomic,
            Some("set") => ElementRelationship::Associative,
            Some("map") => ElementRelationship::Associative,
            _ => {
                match schema.x_kubernetes_patch_strategy.as_deref() {
                    Some("merge") | Some("retainKeys") => ElementRelationship::Associative,
                    _ => ElementRelationship::Atomic,
                }
            }
        }
    }

    /// Get map element relationship from v2 schema.
    fn get_map_element_relationship_v2(&self, schema: &SchemaV2) -> ElementRelationship {
        match schema.x_kubernetes_map_type.as_deref() {
            Some("atomic") => ElementRelationship::Atomic,
            Some("granular") => ElementRelationship::Separable,
            _ => ElementRelationship::Separable, // Default for maps
        }
    }

    /// Get map element relationship from v3 schema.
    fn get_map_element_relationship_v3(&self, schema: &SchemaV3) -> ElementRelationship {
        match schema.x_kubernetes_map_type.as_deref() {
            Some("atomic") => ElementRelationship::Atomic,
            Some("granular") => ElementRelationship::Separable,
            _ => ElementRelationship::Separable,
        }
    }

    /// Get additional properties type for v2.
    fn get_additional_properties_type_v2(
        &mut self,
        schema: &SchemaV2,
        definitions: &BTreeMap<String, SchemaV2>,
        path: &str,
    ) -> TypeRef {
        if let Some(ref additional) = schema.additional_properties {
            match additional.as_ref() {
                AdditionalProperties::Bool(true) => TypeRef {
                    named_type: Some("__untyped_deduced_".to_string()),
                    ..Default::default()
                },
                AdditionalProperties::Bool(false) => TypeRef::default(),
                AdditionalProperties::Schema(s) => {
                    self.v2_schema_to_type_ref(s, definitions, &format!("{}.additionalProperties", path))
                }
            }
        } else {
            TypeRef::default()
        }
    }

    /// Get additional properties type for v3.
    fn get_additional_properties_type_v3(
        &mut self,
        schema: &SchemaV3,
        definitions: &BTreeMap<String, SchemaV3>,
        path: &str,
    ) -> TypeRef {
        if let Some(ref additional) = schema.additional_properties {
            match additional.as_ref() {
                AdditionalProperties::Bool(true) => TypeRef {
                    named_type: Some("__untyped_deduced_".to_string()),
                    ..Default::default()
                },
                AdditionalProperties::Bool(false) => TypeRef::default(),
                AdditionalProperties::Schema(s) => {
                    self.v3_schema_to_type_ref(s, definitions, &format!("{}.additionalProperties", path))
                }
            }
        } else {
            TypeRef::default()
        }
    }

    /// Convert x-kubernetes-unions to Union definitions for v2.
    fn convert_unions_v2(&self, schema: &SchemaV2) -> Vec<Union> {
        schema
            .x_kubernetes_unions
            .as_ref()
            .map(|unions| {
                unions
                    .iter()
                    .map(|u| Union {
                        discriminator: u.discriminator.clone(),
                        fields: u
                            .fields_to_discriminate_by
                            .iter()
                            .map(|(field, disc)| UnionField {
                                field_name: field.clone(),
                                discriminator_value: disc.clone(),
                            })
                            .collect(),
                        deduce_invalid_discriminator: false,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Convert x-kubernetes-unions to Union definitions for v3.
    fn convert_unions_v3(&self, schema: &SchemaV3) -> Vec<Union> {
        schema
            .x_kubernetes_unions
            .as_ref()
            .map(|unions| {
                unions
                    .iter()
                    .map(|u| Union {
                        discriminator: u.discriminator.clone(),
                        fields: u
                            .fields_to_discriminate_by
                            .iter()
                            .map(|(field, disc)| UnionField {
                                field_name: field.clone(),
                                discriminator_value: disc.clone(),
                            })
                            .collect(),
                        deduce_invalid_discriminator: false,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Merge multiple v3 schemas (for allOf).
    fn merge_v3_schemas(
        &mut self,
        schemas: &[SchemaV3],
        definitions: &BTreeMap<String, SchemaV3>,
        path: &str,
    ) -> Atom {
        // Simple merge: collect all properties from all schemas
        let mut merged_properties: BTreeMap<String, SchemaV3> = BTreeMap::new();
        let mut merged_required: Vec<String> = Vec::new();

        for schema in schemas {
            // Handle $ref by resolving it
            if let Some(ref ref_path) = schema.ref_path {
                let type_name = self.extract_type_name_from_ref(ref_path);
                if let Some(ref_schema) = definitions.get(&type_name) {
                    for (name, prop) in &ref_schema.properties {
                        merged_properties.insert(name.clone(), prop.clone());
                    }
                    merged_required.extend(ref_schema.required.clone());
                }
            } else {
                for (name, prop) in &schema.properties {
                    merged_properties.insert(name.clone(), prop.clone());
                }
                merged_required.extend(schema.required.clone());
            }
        }

        // Create a synthetic merged schema
        let merged_schema = SchemaV3 {
            schema_type: Some("object".to_string()),
            properties: merged_properties,
            required: merged_required,
            ..Default::default()
        };

        self.schema_v3_to_atom(&merged_schema, definitions, path)
    }

    /// Add a conversion error.
    fn add_error(&mut self, path: &str, message: &str) {
        self.errors.push(ConversionError {
            path: path.to_string(),
            message: message.to_string(),
        });
    }
}

impl Default for OpenAPIConverter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert an OpenAPI document to SMD schema.
pub fn convert_openapi_to_schema(doc: &OpenAPIDocument) -> ConversionResult {
    let mut converter = OpenAPIConverter::new();
    converter.convert(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_object() {
        let json = r#"{
            "swagger": "2.0",
            "info": {"title": "Test", "version": "1.0"},
            "definitions": {
                "Pet": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    }
                }
            }
        }"#;

        let doc = OpenAPIDocument::from_json(json).unwrap();
        let result = convert_openapi_to_schema(&doc);

        assert!(result.errors.is_empty());
        assert_eq!(result.schema.types.len(), 1);

        let pet_type = &result.schema.types[0];
        assert_eq!(pet_type.name, "Pet");
        assert!(pet_type.atom.map.is_some());

        let map = pet_type.atom.map.as_ref().unwrap();
        assert_eq!(map.fields.len(), 2);
    }

    #[test]
    fn test_convert_array_with_list_type() {
        let json = r##"{
            "swagger": "2.0",
            "info": {"title": "Test", "version": "1.0"},
            "definitions": {
                "ContainerList": {
                    "type": "array",
                    "items": {"$ref": "#/definitions/Container"},
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
        let result = convert_openapi_to_schema(&doc);

        assert!(result.errors.is_empty());

        let container_list = result.schema.types.iter().find(|t| t.name == "ContainerList").unwrap();
        assert!(container_list.atom.list.is_some());

        let list = container_list.atom.list.as_ref().unwrap();
        assert_eq!(list.element_relationship, ElementRelationship::Associative);
        assert_eq!(list.keys, vec!["name"]);
    }

    #[test]
    fn test_convert_atomic_map() {
        let json = r#"{
            "swagger": "2.0",
            "info": {"title": "Test", "version": "1.0"},
            "definitions": {
                "Labels": {
                    "type": "object",
                    "additionalProperties": {"type": "string"},
                    "x-kubernetes-map-type": "atomic"
                }
            }
        }"#;

        let doc = OpenAPIDocument::from_json(json).unwrap();
        let result = convert_openapi_to_schema(&doc);

        assert!(result.errors.is_empty());

        let labels = result.schema.types.iter().find(|t| t.name == "Labels").unwrap();
        assert!(labels.atom.map.is_some());

        let map = labels.atom.map.as_ref().unwrap();
        assert_eq!(map.element_relationship, ElementRelationship::Atomic);
    }

    #[test]
    fn test_convert_v3_document() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": {"title": "Test", "version": "1.0"},
            "components": {
                "schemas": {
                    "Pet": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "tags": {
                                "type": "array",
                                "items": {"type": "string"},
                                "x-kubernetes-list-type": "set"
                            }
                        }
                    }
                }
            }
        }"#;

        let doc = OpenAPIDocument::from_json(json).unwrap();
        let result = convert_openapi_to_schema(&doc);

        assert!(result.errors.is_empty());
        assert_eq!(result.schema.types.len(), 1);

        let pet = &result.schema.types[0];
        assert_eq!(pet.name, "Pet");
    }

    #[test]
    fn test_convert_preserve_unknown_fields() {
        let json = r#"{
            "swagger": "2.0",
            "info": {"title": "Test", "version": "1.0"},
            "definitions": {
                "RawExtension": {
                    "type": "object",
                    "x-kubernetes-preserve-unknown-fields": true
                }
            }
        }"#;

        let doc = OpenAPIDocument::from_json(json).unwrap();
        let result = convert_openapi_to_schema(&doc);

        assert!(result.errors.is_empty());

        let raw_ext = result.schema.types.iter().find(|t| t.name == "RawExtension").unwrap();
        assert!(raw_ext.atom.map.is_some());

        let map = raw_ext.atom.map.as_ref().unwrap();
        assert!(map.element_type.named_type.as_ref().unwrap().contains("untyped"));
    }
}
