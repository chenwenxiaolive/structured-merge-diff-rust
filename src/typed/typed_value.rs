//! TypedValue implementation.

use crate::fieldpath::{Path, PathElement, Set};
use crate::schema::{ElementRelationship, Schema, Scalar, TypeRef};
use crate::value::{Field, FieldList, Value};
use super::comparison::Comparison;
use super::validation::{ValidationError, ValidationErrors, ValidationOption};

/// TypedValue is a Value paired with its schema and type.
#[derive(Debug, Clone)]
pub struct TypedValue {
    value: Value,
    type_ref: TypeRef,
    schema: Schema,
}

/// Creates a new TypedValue after validating it conforms to the schema.
pub fn as_typed(
    value: Value,
    schema: &Schema,
    type_ref: TypeRef,
    opts: &[ValidationOption],
) -> Result<TypedValue, ValidationErrors> {
    let tv = TypedValue {
        value,
        type_ref,
        schema: schema.clone(),
    };
    tv.validate(opts)?;
    Ok(tv)
}

/// Creates a new TypedValue without validation.
/// Use this only when validation has already been done.
pub fn as_typed_unvalidated(value: Value, schema: &Schema, type_ref: TypeRef) -> TypedValue {
    TypedValue {
        value,
        type_ref,
        schema: schema.clone(),
    }
}

impl TypedValue {
    /// Creates a new TypedValue.
    pub fn new(value: Value, schema: Schema, type_ref: TypeRef) -> Self {
        TypedValue {
            value,
            type_ref,
            schema,
        }
    }

    /// Returns a reference to the underlying value.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Returns a mutable reference to the underlying value.
    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    /// Consumes the TypedValue and returns the underlying value.
    pub fn into_value(self) -> Value {
        self.value
    }

    /// Returns a reference to the type reference.
    pub fn type_ref(&self) -> &TypeRef {
        &self.type_ref
    }

    /// Returns a reference to the schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Validates the value against the schema.
    pub fn validate(&self, opts: &[ValidationOption]) -> Result<(), ValidationErrors> {
        let allow_duplicates = opts.contains(&ValidationOption::AllowDuplicates);
        let mut errors = ValidationErrors::new();

        self.validate_value(&self.value, &self.type_ref, Path::new(), allow_duplicates, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_value(
        &self,
        value: &Value,
        type_ref: &TypeRef,
        path: Path,
        allow_duplicates: bool,
        errors: &mut ValidationErrors,
    ) {
        // Resolve the type reference
        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => {
                if let Some(ref name) = type_ref.named_type {
                    errors.add(ValidationError::schema_error(format!(
                        "no type found matching: {}",
                        name
                    )));
                }
                return;
            }
        };

        // Validate based on the atom type
        if let Some(ref scalar) = atom.scalar {
            self.validate_scalar(value, scalar, &path, errors);
        } else if let Some(ref list) = atom.list {
            self.validate_list(value, list, path, allow_duplicates, errors);
        } else if let Some(ref map) = atom.map {
            self.validate_map(value, map, path, allow_duplicates, errors);
        }
    }

    fn validate_scalar(
        &self,
        value: &Value,
        scalar: &Scalar,
        path: &Path,
        errors: &mut ValidationErrors,
    ) {
        if value.is_null() {
            return; // null is always valid
        }

        let valid = match scalar {
            Scalar::Numeric => value.is_int() || value.is_float(),
            Scalar::String => value.is_string(),
            Scalar::Boolean => value.is_bool(),
            Scalar::Untyped => value.is_int() || value.is_float() || value.is_string() || value.is_bool(),
        };

        if !valid {
            let expected = match scalar {
                Scalar::Numeric => "numeric",
                Scalar::String => "string",
                Scalar::Boolean => "boolean",
                Scalar::Untyped => "scalar",
            };
            let actual = match value {
                Value::Null => "null",
                Value::Bool(_) => "boolean",
                Value::Int(_) => "int",
                Value::Float(_) => "float",
                Value::String(_) => "string",
                Value::List(_) => "list",
                Value::Map(_) => "map",
            };
            errors.add(ValidationError::type_mismatch(
                format!("{}", path),
                expected,
                actual,
            ));
        }
    }

    fn validate_list(
        &self,
        value: &Value,
        list: &crate::schema::List,
        path: Path,
        allow_duplicates: bool,
        errors: &mut ValidationErrors,
    ) {
        let items = match value {
            Value::Null => return,
            Value::List(l) => l,
            _ => {
                errors.add(ValidationError::type_mismatch(
                    format!("{}", path),
                    "list",
                    value_type_name(value),
                ));
                return;
            }
        };

        // Track keys for duplicate detection in associative lists
        let mut seen_keys = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let pe = if list.element_relationship == ElementRelationship::Associative {
                // For associative lists, compute key
                match self.list_item_to_key(item, list) {
                    Ok(key) => {
                        if !allow_duplicates && seen_keys.contains(&key) {
                            errors.add(ValidationError::duplicate_key(
                                format!("{}", path),
                                format!("{:?}", key),
                            ));
                        }
                        seen_keys.push(key.clone());
                        PathElement::Key(key)
                    }
                    Err(e) => {
                        errors.add(e);
                        PathElement::index(i as i32)
                    }
                }
            } else {
                PathElement::index(i as i32)
            };

            let item_path = path.with(pe);
            self.validate_value(item, &list.element_type, item_path, allow_duplicates, errors);
        }
    }

    fn validate_map(
        &self,
        value: &Value,
        map: &crate::schema::Map,
        path: Path,
        allow_duplicates: bool,
        errors: &mut ValidationErrors,
    ) {
        let fields = match value {
            Value::Null => return,
            Value::Map(m) => m,
            _ => {
                errors.add(ValidationError::type_mismatch(
                    format!("{}", path),
                    "map",
                    value_type_name(value),
                ));
                return;
            }
        };

        for (key, val) in fields.iter() {
            let pe = PathElement::field_name(key.clone());
            let field_path = path.with(pe);

            // Find the field type
            let field_type = if let Some(field) = map.find_field(key) {
                field.field_type.clone()
            } else {
                // Check if unknown fields are allowed (element_type is set)
                if map.element_type.named_type.is_some() || map.element_type.inlined.scalar.is_some()
                    || map.element_type.inlined.list.is_some() || map.element_type.inlined.map.is_some() {
                    map.element_type.clone()
                } else {
                    errors.add(ValidationError::unknown_field(
                        format!("{}", path),
                        key.clone(),
                    ));
                    continue;
                }
            };

            self.validate_value(val, &field_type, field_path, allow_duplicates, errors);
        }
    }

    fn list_item_to_key(
        &self,
        item: &Value,
        list: &crate::schema::List,
    ) -> Result<FieldList, ValidationError> {
        if list.keys.is_empty() {
            // Set semantics - use the value itself
            return Ok(FieldList::with_fields(vec![Field {
                name: String::new(),
                value: item.clone(),
            }]));
        }

        // Associative list - extract key fields
        let map = match item {
            Value::Map(m) => m,
            _ => {
                return Err(ValidationError::invalid_value(
                    "",
                    "expected map for associative list item",
                ));
            }
        };

        let mut fields = Vec::new();
        for key_name in &list.keys {
            match map.get(key_name) {
                Some(v) => {
                    fields.push(Field {
                        name: key_name.clone(),
                        value: v.clone(),
                    });
                }
                None => {
                    return Err(ValidationError::missing_field("", key_name.clone()));
                }
            }
        }

        Ok(FieldList::with_fields(fields))
    }

    /// Converts the typed value to a field set representing all leaf paths.
    pub fn to_field_set(&self) -> Result<Set, ValidationErrors> {
        let mut set = Set::new();
        let mut errors = ValidationErrors::new();

        self.collect_field_set(&self.value, &self.type_ref, Path::new(), &mut set, &mut errors);

        if errors.is_empty() {
            Ok(set)
        } else {
            Err(errors)
        }
    }

    fn collect_field_set(
        &self,
        value: &Value,
        type_ref: &TypeRef,
        path: Path,
        set: &mut Set,
        errors: &mut ValidationErrors,
    ) {
        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => return,
        };

        if let Some(_) = atom.scalar {
            // Scalars are leaf nodes
            if !path.is_empty() {
                set.insert(&path);
            }
        } else if let Some(ref list) = atom.list {
            if list.element_relationship == ElementRelationship::Atomic {
                // Atomic lists are leaves
                if !path.is_empty() {
                    set.insert(&path);
                }
            } else if let Value::List(items) = value {
                for (i, item) in items.iter().enumerate() {
                    let pe = if list.element_relationship == ElementRelationship::Associative {
                        match self.list_item_to_key(item, list) {
                            Ok(key) => PathElement::Key(key),
                            Err(_) => PathElement::index(i as i32),
                        }
                    } else {
                        PathElement::index(i as i32)
                    };
                    let item_path = path.with(pe);
                    self.collect_field_set(item, &list.element_type, item_path, set, errors);
                }
            }
        } else if let Some(ref map) = atom.map {
            if map.element_relationship == ElementRelationship::Atomic {
                // Atomic maps are leaves
                if !path.is_empty() {
                    set.insert(&path);
                }
            } else if let Value::Map(fields) = value {
                for (key, val) in fields.iter() {
                    let pe = PathElement::field_name(key.clone());
                    let field_path = path.with(pe);

                    let field_type = if let Some(field) = map.find_field(key) {
                        field.field_type.clone()
                    } else {
                        map.element_type.clone()
                    };

                    self.collect_field_set(val, &field_type, field_path, set, errors);
                }
            }
        }
    }

    /// Compares this TypedValue with another.
    pub fn compare(&self, rhs: &TypedValue) -> Result<Comparison, ValidationErrors> {
        // Verify same schema/type
        if self.type_ref != rhs.type_ref {
            return Err(ValidationErrors::from_error(ValidationError::schema_error(
                "expected objects of the same type",
            )));
        }

        let mut comparison = Comparison::new();
        self.compare_values(
            &self.value,
            &rhs.value,
            &self.type_ref,
            Path::new(),
            &mut comparison,
        );

        Ok(comparison)
    }

    fn compare_values(
        &self,
        lhs: &Value,
        rhs: &Value,
        type_ref: &TypeRef,
        path: Path,
        comparison: &mut Comparison,
    ) {
        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => return,
        };

        // Handle scalar comparison
        if let Some(_) = atom.scalar {
            if lhs != rhs {
                comparison.modified.insert(&path);
            }
            return;
        }

        // Handle list comparison
        if let Some(ref list) = atom.list {
            self.compare_lists(lhs, rhs, list, path, comparison);
            return;
        }

        // Handle map comparison
        if let Some(ref map) = atom.map {
            self.compare_maps(lhs, rhs, map, path, comparison);
        }
    }

    fn compare_lists(
        &self,
        lhs: &Value,
        rhs: &Value,
        list: &crate::schema::List,
        path: Path,
        comparison: &mut Comparison,
    ) {
        // For atomic lists, compare as a whole
        if list.element_relationship == ElementRelationship::Atomic {
            if lhs != rhs {
                comparison.modified.insert(&path);
            }
            return;
        }

        let lhs_items = match lhs {
            Value::List(l) => l.as_slice(),
            Value::Null => &[],
            _ => return,
        };
        let rhs_items = match rhs {
            Value::List(r) => r.as_slice(),
            Value::Null => &[],
            _ => return,
        };

        // Build index maps for associative lists
        let mut lhs_by_key = std::collections::HashMap::new();
        let mut rhs_by_key = std::collections::HashMap::new();

        for (i, item) in lhs_items.iter().enumerate() {
            let key = if list.element_relationship == ElementRelationship::Associative {
                self.list_item_to_key(item, list).ok()
            } else {
                None
            };
            let pe = key.map(PathElement::Key).unwrap_or_else(|| PathElement::index(i as i32));
            lhs_by_key.insert(pe, item);
        }

        for (i, item) in rhs_items.iter().enumerate() {
            let key = if list.element_relationship == ElementRelationship::Associative {
                self.list_item_to_key(item, list).ok()
            } else {
                None
            };
            let pe = key.map(PathElement::Key).unwrap_or_else(|| PathElement::index(i as i32));
            rhs_by_key.insert(pe, item);
        }

        // Find removed items (in lhs but not rhs)
        for (pe, _) in &lhs_by_key {
            if !rhs_by_key.contains_key(pe) {
                comparison.removed.insert(&path.with(pe.clone()));
            }
        }

        // Find added items (in rhs but not lhs) and modified items
        for (pe, rhs_item) in &rhs_by_key {
            match lhs_by_key.get(pe) {
                None => {
                    comparison.added.insert(&path.with(pe.clone()));
                }
                Some(lhs_item) => {
                    let item_path = path.with(pe.clone());
                    self.compare_values(lhs_item, rhs_item, &list.element_type, item_path, comparison);
                }
            }
        }
    }

    fn compare_maps(
        &self,
        lhs: &Value,
        rhs: &Value,
        map: &crate::schema::Map,
        path: Path,
        comparison: &mut Comparison,
    ) {
        // For atomic maps, compare as a whole
        if map.element_relationship == ElementRelationship::Atomic {
            if lhs != rhs {
                comparison.modified.insert(&path);
            }
            return;
        }

        let lhs_fields = match lhs {
            Value::Map(m) => m,
            Value::Null => return,
            _ => return,
        };
        let rhs_fields = match rhs {
            Value::Map(m) => m,
            Value::Null => return,
            _ => return,
        };

        // Find removed fields
        for (key, _) in lhs_fields.iter() {
            if !rhs_fields.has(key) {
                let pe = PathElement::field_name(key.clone());
                comparison.removed.insert(&path.with(pe));
            }
        }

        // Find added and modified fields
        for (key, rhs_val) in rhs_fields.iter() {
            let pe = PathElement::field_name(key.clone());
            let field_path = path.with(pe);

            match lhs_fields.get(key) {
                None => {
                    comparison.added.insert(&field_path);
                }
                Some(lhs_val) => {
                    let field_type = if let Some(field) = map.find_field(key) {
                        field.field_type.clone()
                    } else {
                        map.element_type.clone()
                    };
                    self.compare_values(lhs_val, rhs_val, &field_type, field_path, comparison);
                }
            }
        }
    }

    /// Removes items from the value based on the provided set of paths.
    pub fn remove_items(&self, items: &Set) -> TypedValue {
        let new_value = self.remove_items_from_value(&self.value, &self.type_ref, items, Path::new());
        TypedValue {
            value: new_value,
            type_ref: self.type_ref.clone(),
            schema: self.schema.clone(),
        }
    }

    fn remove_items_from_value(
        &self,
        value: &Value,
        type_ref: &TypeRef,
        items: &Set,
        path: Path,
    ) -> Value {
        // If this exact path should be removed, return null
        if items.has(&path) {
            return Value::Null;
        }

        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => return value.clone(),
        };

        // Scalars don't have children to remove
        if atom.scalar.is_some() {
            return value.clone();
        }

        // Handle lists
        if let (Some(ref list), Value::List(values)) = (&atom.list, value) {
            let mut new_values = Vec::new();
            for (i, item) in values.iter().enumerate() {
                let pe = if list.element_relationship == ElementRelationship::Associative {
                    match self.list_item_to_key(item, list) {
                        Ok(key) => PathElement::Key(key),
                        Err(_) => PathElement::index(i as i32),
                    }
                } else {
                    PathElement::index(i as i32)
                };
                let item_path = path.with(pe);

                if !items.has(&item_path) {
                    let new_item = self.remove_items_from_value(item, &list.element_type, items, item_path);
                    new_values.push(new_item);
                }
            }
            return Value::List(new_values);
        }

        // Handle maps
        if let (Some(ref map), Value::Map(fields)) = (&atom.map, value) {
            let mut new_map = crate::value::Map::new();
            for (key, val) in fields.iter() {
                let pe = PathElement::field_name(key.clone());
                let field_path = path.with(pe);

                if !items.has(&field_path) {
                    let field_type = if let Some(field) = map.find_field(key) {
                        field.field_type.clone()
                    } else {
                        map.element_type.clone()
                    };
                    let new_val = self.remove_items_from_value(val, &field_type, items, field_path);
                    new_map.set(key.clone(), new_val);
                }
            }
            return Value::Map(new_map);
        }

        value.clone()
    }

    /// Extracts only the items specified in the set.
    pub fn extract_items(&self, items: &Set) -> TypedValue {
        let new_value = self.extract_items_from_value(&self.value, &self.type_ref, items, Path::new());
        TypedValue {
            value: new_value,
            type_ref: self.type_ref.clone(),
            schema: self.schema.clone(),
        }
    }

    fn extract_items_from_value(
        &self,
        value: &Value,
        type_ref: &TypeRef,
        items: &Set,
        path: Path,
    ) -> Value {
        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => return Value::Null,
        };

        // If this exact path should be included, return the value
        if items.has(&path) {
            return value.clone();
        }

        // Scalars - only include if the path is in items
        if atom.scalar.is_some() {
            return Value::Null;
        }

        // Handle lists
        if let (Some(ref list), Value::List(values)) = (&atom.list, value) {
            let mut new_values = Vec::new();
            for (i, item) in values.iter().enumerate() {
                let pe = if list.element_relationship == ElementRelationship::Associative {
                    match self.list_item_to_key(item, list) {
                        Ok(key) => PathElement::Key(key),
                        Err(_) => PathElement::index(i as i32),
                    }
                } else {
                    PathElement::index(i as i32)
                };
                let item_path = path.with(pe);

                let new_item = self.extract_items_from_value(item, &list.element_type, items, item_path);
                if !matches!(new_item, Value::Null) {
                    new_values.push(new_item);
                }
            }
            if new_values.is_empty() {
                return Value::Null;
            }
            return Value::List(new_values);
        }

        // Handle maps
        if let (Some(ref map), Value::Map(fields)) = (&atom.map, value) {
            let mut new_map = crate::value::Map::new();
            for (key, val) in fields.iter() {
                let pe = PathElement::field_name(key.clone());
                let field_path = path.with(pe);

                let field_type = if let Some(field) = map.find_field(key) {
                    field.field_type.clone()
                } else {
                    map.element_type.clone()
                };
                let new_val = self.extract_items_from_value(val, &field_type, items, field_path);
                if !matches!(new_val, Value::Null) {
                    new_map.set(key.clone(), new_val);
                }
            }
            if new_map.is_empty() {
                return Value::Null;
            }
            return Value::Map(new_map);
        }

        Value::Null
    }

    /// Merges another TypedValue into this one.
    ///
    /// The merge strategy is "keep RHS" - if both lhs (self) and rhs have a value
    /// at the same path, the rhs value is used. For maps, fields are recursively
    /// merged. For atomic lists/maps, they are replaced entirely.
    pub fn merge(&self, rhs: &TypedValue) -> Result<TypedValue, ValidationErrors> {
        if self.type_ref != rhs.type_ref {
            return Err(ValidationErrors::from_error(ValidationError::schema_error(
                "expected objects of the same type",
            )));
        }

        let new_value = self.merge_values(&self.value, &rhs.value, &self.type_ref);

        Ok(TypedValue {
            value: new_value,
            type_ref: self.type_ref.clone(),
            schema: self.schema.clone(),
        })
    }

    fn merge_values(&self, lhs: &Value, rhs: &Value, type_ref: &TypeRef) -> Value {
        // If rhs is null, keep lhs
        if matches!(rhs, Value::Null) {
            return lhs.clone();
        }

        // If lhs is null, use rhs
        if matches!(lhs, Value::Null) {
            return rhs.clone();
        }

        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => return rhs.clone(),
        };

        // Scalars - keep rhs
        if atom.scalar.is_some() {
            return rhs.clone();
        }

        // Atomic lists and maps - replace entirely
        if let Some(ref list) = atom.list {
            if list.element_relationship == ElementRelationship::Atomic {
                return rhs.clone();
            }

            // Non-atomic lists - merge by key (for associative) or by index
            if let (Value::List(lhs_items), Value::List(rhs_items)) = (lhs, rhs) {
                return self.merge_lists(lhs_items, rhs_items, list);
            }
            return rhs.clone();
        }

        if let Some(ref map) = atom.map {
            if map.element_relationship == ElementRelationship::Atomic {
                return rhs.clone();
            }

            // Non-atomic maps - merge fields recursively
            if let (Value::Map(lhs_fields), Value::Map(rhs_fields)) = (lhs, rhs) {
                return self.merge_maps(lhs_fields, rhs_fields, map);
            }
            return rhs.clone();
        }

        rhs.clone()
    }

    fn merge_lists(&self, lhs: &[Value], rhs: &[Value], list: &crate::schema::List) -> Value {
        if list.element_relationship == ElementRelationship::Associative {
            // Build index by key
            let mut merged: std::collections::HashMap<FieldList, Value> = std::collections::HashMap::new();
            let mut order: Vec<FieldList> = Vec::new();

            // Add lhs items
            for item in lhs {
                if let Ok(key) = self.list_item_to_key(item, list) {
                    if !merged.contains_key(&key) {
                        order.push(key.clone());
                    }
                    merged.insert(key, item.clone());
                }
            }

            // Merge rhs items (replaces or adds)
            for item in rhs {
                if let Ok(key) = self.list_item_to_key(item, list) {
                    if let Some(lhs_item) = merged.get(&key) {
                        // Merge the items recursively
                        let merged_item = self.merge_values(lhs_item, item, &list.element_type);
                        if !merged.contains_key(&key) {
                            order.push(key.clone());
                        }
                        merged.insert(key, merged_item);
                    } else {
                        order.push(key.clone());
                        merged.insert(key, item.clone());
                    }
                }
            }

            // Reconstruct list in order
            let result: Vec<Value> = order.into_iter()
                .filter_map(|k| merged.remove(&k))
                .collect();
            Value::List(result)
        } else {
            // Non-associative lists - just use rhs entirely
            Value::List(rhs.to_vec())
        }
    }

    fn merge_maps(&self, lhs: &crate::value::Map, rhs: &crate::value::Map, map: &crate::schema::Map) -> Value {
        let mut result = crate::value::Map::new();

        // Copy all lhs fields
        for (key, val) in lhs.iter() {
            result.set(key.clone(), val.clone());
        }

        // Merge rhs fields
        for (key, rhs_val) in rhs.iter() {
            let field_type = if let Some(field) = map.find_field(key) {
                field.field_type.clone()
            } else {
                map.element_type.clone()
            };

            let new_val = if let Some(lhs_val) = lhs.get(key) {
                self.merge_values(lhs_val, rhs_val, &field_type)
            } else {
                rhs_val.clone()
            };
            result.set(key.clone(), new_val);
        }

        Value::Map(result)
    }

    /// Creates an empty TypedValue with the same schema and type.
    pub fn empty(&self) -> TypedValue {
        TypedValue {
            value: Value::Null,
            type_ref: self.type_ref.clone(),
            schema: self.schema.clone(),
        }
    }
}

fn value_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::List(_) => "list",
        Value::Map(_) => "map",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Atom, TypeDef};

    #[test]
    fn test_typed_value_creation() {
        let value = Value::Map(crate::value::Map::new());
        let schema = Schema::new();
        let type_ref = TypeRef::default();

        let typed = TypedValue::new(value.clone(), schema, type_ref);
        assert_eq!(typed.value(), &value);
    }

    #[test]
    fn test_typed_value_compare_scalars() {
        let schema = Schema::with_types(vec![TypeDef {
            name: "string".to_string(),
            atom: Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            },
        }]);

        let type_ref = TypeRef {
            named_type: Some("string".to_string()),
            ..Default::default()
        };

        let tv1 = TypedValue::new(Value::String("hello".into()), schema.clone(), type_ref.clone());
        let tv2 = TypedValue::new(Value::String("world".into()), schema.clone(), type_ref.clone());

        let comparison = tv1.compare(&tv2).unwrap();
        assert!(!comparison.is_same());
        // Root scalar is modified
    }

    #[test]
    fn test_typed_value_compare_same() {
        let schema = Schema::with_types(vec![TypeDef {
            name: "string".to_string(),
            atom: Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            },
        }]);

        let type_ref = TypeRef {
            named_type: Some("string".to_string()),
            ..Default::default()
        };

        let tv1 = TypedValue::new(Value::String("same".into()), schema.clone(), type_ref.clone());
        let tv2 = TypedValue::new(Value::String("same".into()), schema.clone(), type_ref.clone());

        let comparison = tv1.compare(&tv2).unwrap();
        assert!(comparison.is_same());
    }

    #[test]
    fn test_validate_scalar() {
        let schema = Schema::with_types(vec![TypeDef {
            name: "string".to_string(),
            atom: Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            },
        }]);

        let type_ref = TypeRef {
            named_type: Some("string".to_string()),
            ..Default::default()
        };

        // Valid string
        let tv = TypedValue::new(Value::String("hello".into()), schema.clone(), type_ref.clone());
        assert!(tv.validate(&[]).is_ok());

        // Invalid - int instead of string
        let tv = TypedValue::new(Value::Int(42), schema.clone(), type_ref.clone());
        assert!(tv.validate(&[]).is_err());
    }
}
