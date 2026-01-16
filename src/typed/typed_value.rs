//! TypedValue implementation.

use crate::fieldpath::{Path, PathElement, Set};
use crate::schema::{ElementRelationship, Schema, Scalar, TypeRef};
use crate::value::{Field, FieldList, Map, Value};
use super::comparison::Comparison;
use super::validation::{ValidationError, ValidationErrors, ValidationOption};

/// Converts a serde_json::Value to our Value type.
fn json_value_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            Value::List(arr.iter().map(json_value_to_value).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut map = Map::new();
            for (k, v) in obj {
                map.set(k.clone(), json_value_to_value(v));
            }
            Value::Map(map)
        }
    }
}

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

        // Validate based on the value type AND the available schema types
        // This handles union types where an atom can have scalar, list, and map all defined
        match value {
            Value::Null => {
                // null is always valid
            }
            Value::Bool(_) | Value::Int(_) | Value::Float(_) | Value::String(_) => {
                if let Some(ref scalar) = atom.scalar {
                    self.validate_scalar(value, scalar, &path, errors);
                } else {
                    // No scalar type defined, try to see if it fits list or map
                    errors.add(ValidationError::type_mismatch(
                        format!("{}", path),
                        if atom.list.is_some() { "list" } else if atom.map.is_some() { "map" } else { "unknown" },
                        value_type_name(value),
                    ));
                }
            }
            Value::List(_) => {
                if let Some(ref list) = atom.list {
                    self.validate_list(value, list, path, allow_duplicates, errors);
                } else {
                    errors.add(ValidationError::type_mismatch(
                        format!("{}", path),
                        if atom.scalar.is_some() { "scalar" } else if atom.map.is_some() { "map" } else { "unknown" },
                        "list",
                    ));
                }
            }
            Value::Map(_) => {
                if let Some(ref map) = atom.map {
                    self.validate_map(value, map, path, allow_duplicates, errors);
                } else {
                    errors.add(ValidationError::type_mismatch(
                        format!("{}", path),
                        if atom.scalar.is_some() { "scalar" } else if atom.list.is_some() { "list" } else { "unknown" },
                        "map",
                    ));
                }
            }
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
                    // Try to get default value from schema
                    if let Some(default_val) = self.get_associative_key_default(list, key_name) {
                        fields.push(Field {
                            name: key_name.clone(),
                            value: default_val,
                        });
                    }
                    // If no default, don't add this key to the list
                    // This allows partial keys where only some key fields have defaults
                }
            }
        }

        // If we have keys defined but couldn't find any key values (even with defaults),
        // that's an error
        if !list.keys.is_empty() && fields.is_empty() {
            return Err(ValidationError::invalid_value(
                "",
                format!(
                    "associative list with keys has an element that omits all key fields {:?} (and doesn't have default values for any key fields)",
                    list.keys
                ),
            ));
        }

        Ok(FieldList::with_fields(fields))
    }

    /// Gets the default value for an associative list key field from the schema.
    fn get_associative_key_default(&self, list: &crate::schema::List, field_name: &str) -> Option<Value> {
        // Resolve the list's element type to get the map schema
        let atom = self.schema.resolve(&list.element_type)?;
        let map_schema = atom.map.as_ref()?;

        // Find the field in the map schema
        let field = map_schema.find_field(field_name)?;

        // Return the default value if it exists, converting from serde_json::Value to our Value
        field.default.as_ref().map(|default| json_value_to_value(default))
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

        // Check value type first to handle "sum types" like deduced schema
        // which have scalar, list, AND map all defined
        match value {
            Value::Null => {
                // Null values are leaves - insert the path regardless of schema type
                if !path.is_empty() {
                    set.insert(&path);
                }
            }
            Value::Map(fields) => {
                if let Some(ref map) = atom.map {
                    if map.element_relationship == ElementRelationship::Atomic {
                        // Atomic maps are leaves
                        if !path.is_empty() {
                            set.insert(&path);
                        }
                    } else {
                        // Non-atomic maps: recurse into fields
                        // For sum types (deduced schema), also insert the map path itself
                        // A sum type has both scalar and map defined
                        let is_sum_type = atom.scalar.is_some();
                        let is_associative = map.element_relationship == ElementRelationship::Associative;
                        if is_sum_type && !path.is_empty() {
                            set.insert(&path);
                        } else if fields.is_empty() && !path.is_empty() {
                            // For regular schemas, only insert if empty (shouldn't happen normally)
                            set.insert(&path);
                        }
                        for (key, val) in fields.iter() {
                            let pe = PathElement::field_name(key.clone());
                            let field_path = path.with(pe);

                            let field_type = if let Some(field) = map.find_field(key) {
                                field.field_type.clone()
                            } else {
                                map.element_type.clone()
                            };

                            self.collect_field_set(val, &field_type, field_path.clone(), set, errors);

                            // For associative maps with element_type (not explicit fields),
                            // insert each key's path similar to how we handle associative lists
                            if is_associative && map.fields.is_empty() && map.element_type.named_type.is_some() {
                                set.insert(&field_path);
                            }
                        }
                    }
                } else if atom.scalar.is_some() {
                    // Fallback to scalar treatment
                    if !path.is_empty() {
                        set.insert(&path);
                    }
                }
            }
            Value::List(items) => {
                if let Some(ref list) = atom.list {
                    if list.element_relationship == ElementRelationship::Atomic {
                        // Atomic lists are leaves
                        if !path.is_empty() {
                            set.insert(&path);
                        }
                    } else {
                        for (i, item) in items.iter().enumerate() {
                            let pe = if list.element_relationship == ElementRelationship::Associative {
                                if list.keys.is_empty() {
                                    // Set semantics - use the value as the path element
                                    PathElement::value(item.clone())
                                } else {
                                    // Keyed associative list
                                    match self.list_item_to_key(item, list) {
                                        Ok(key) => PathElement::Key(key),
                                        Err(_) => PathElement::index(i as i32),
                                    }
                                }
                            } else {
                                PathElement::index(i as i32)
                            };
                            let item_path = path.with(pe);
                            self.collect_field_set(item, &list.element_type, item_path.clone(), set, errors);
                            // For keyed associative lists, also insert the item path itself
                            if list.element_relationship == ElementRelationship::Associative && !list.keys.is_empty() {
                                set.insert(&item_path);
                            }
                        }
                    }
                } else if atom.scalar.is_some() {
                    // Fallback to scalar treatment
                    if !path.is_empty() {
                        set.insert(&path);
                    }
                }
            }
            _ => {
                // Scalar values (String, Int, Float, Bool, Null)
                if atom.scalar.is_some() {
                    if !path.is_empty() {
                        set.insert(&path);
                    }
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

        // Check value types first to handle "sum types" like deduced schema
        match (lhs, rhs) {
            (Value::Map(_), Value::Map(_)) => {
                if let Some(ref map) = atom.map {
                    self.compare_maps(lhs, rhs, map, path, comparison);
                } else if lhs != rhs {
                    comparison.modified.insert(&path);
                }
            }
            (Value::List(_), Value::List(_)) => {
                if let Some(ref list) = atom.list {
                    self.compare_lists(lhs, rhs, list, path, comparison);
                } else if lhs != rhs {
                    comparison.modified.insert(&path);
                }
            }
            _ => {
                // Type mismatch or scalar comparison
                if lhs != rhs {
                    comparison.modified.insert(&path);

                    // For type changes, track nested paths as added/removed
                    // If LHS is a map, all its nested paths are "removed"
                    if let Value::Map(_) = lhs {
                        if atom.map.is_some() {
                            self.collect_all_paths(lhs, type_ref, path.clone(), &mut comparison.removed);
                        }
                    }
                    // If RHS is a map, all its nested paths are "added"
                    if let Value::Map(_) = rhs {
                        if atom.map.is_some() {
                            self.collect_all_paths(rhs, type_ref, path.clone(), &mut comparison.added);
                        }
                    }
                }
            }
        }
    }

    /// Collects all nested paths from a value into a set.
    fn collect_all_paths(
        &self,
        value: &Value,
        type_ref: &TypeRef,
        path: Path,
        set: &mut Set,
    ) {
        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => return,
        };

        match value {
            Value::Map(fields) => {
                if let Some(ref map) = atom.map {
                    if map.element_relationship == ElementRelationship::Atomic {
                        return; // Atomic maps are leaves, already tracked at parent
                    }
                    for (key, val) in fields.iter() {
                        let pe = PathElement::field_name(key.clone());
                        let field_path = path.with(pe);

                        // Insert the field path
                        set.insert(&field_path);

                        let field_type = if let Some(field) = map.find_field(key) {
                            field.field_type.clone()
                        } else {
                            map.element_type.clone()
                        };

                        self.collect_all_paths(val, &field_type, field_path, set);
                    }
                }
            }
            Value::List(items) => {
                if let Some(ref list) = atom.list {
                    if list.element_relationship == ElementRelationship::Atomic {
                        return; // Atomic lists are leaves
                    }
                    for (i, item) in items.iter().enumerate() {
                        let pe = if list.element_relationship == ElementRelationship::Associative {
                            if list.keys.is_empty() {
                                PathElement::value(item.clone())
                            } else {
                                match self.list_item_to_key(item, list) {
                                    Ok(key) => PathElement::Key(key),
                                    Err(_) => PathElement::index(i as i32),
                                }
                            }
                        } else {
                            PathElement::index(i as i32)
                        };
                        let item_path = path.with(pe);
                        set.insert(&item_path);
                        self.collect_all_paths(item, &list.element_type, item_path, set);
                    }
                }
            }
            _ => {} // Scalars don't have nested paths
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
            let pe = if list.element_relationship == ElementRelationship::Associative {
                if list.keys.is_empty() {
                    // Set semantics - use the value as the path element
                    PathElement::value(item.clone())
                } else {
                    // Keyed associative list
                    match self.list_item_to_key(item, list) {
                        Ok(key) => PathElement::Key(key),
                        Err(_) => PathElement::index(i as i32),
                    }
                }
            } else {
                PathElement::index(i as i32)
            };
            lhs_by_key.insert(pe, item);
        }

        for (i, item) in rhs_items.iter().enumerate() {
            let pe = if list.element_relationship == ElementRelationship::Associative {
                if list.keys.is_empty() {
                    // Set semantics - use the value as the path element
                    PathElement::value(item.clone())
                } else {
                    // Keyed associative list
                    match self.list_item_to_key(item, list) {
                        Ok(key) => PathElement::Key(key),
                        Err(_) => PathElement::index(i as i32),
                    }
                }
            } else {
                PathElement::index(i as i32)
            };
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

        // Handle null vs non-null as modification
        let lhs_is_null = matches!(lhs, Value::Null);
        let rhs_is_null = matches!(rhs, Value::Null);

        if lhs_is_null != rhs_is_null {
            // One is null and the other is not - this is a modification
            comparison.modified.insert(&path);
        }

        let lhs_fields = match lhs {
            Value::Map(m) => m,
            Value::Null => {
                // rhs must be a map, so all its fields are added
                if let Value::Map(rhs_map) = rhs {
                    for (key, _) in rhs_map.iter() {
                        let pe = PathElement::field_name(key.clone());
                        comparison.added.insert(&path.with(pe));
                    }
                }
                return;
            },
            _ => return,
        };
        let rhs_fields = match rhs {
            Value::Map(m) => m,
            Value::Null => {
                // lhs must be a map, so all its fields are removed
                for (key, _) in lhs_fields.iter() {
                    let pe = PathElement::field_name(key.clone());
                    comparison.removed.insert(&path.with(pe));
                }
                return;
            },
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

            let field_type = if let Some(field) = map.find_field(key) {
                field.field_type.clone()
            } else {
                map.element_type.clone()
            };

            match lhs_fields.get(key) {
                None => {
                    comparison.added.insert(&field_path);
                    // Recursively collect all nested paths from the added field
                    self.collect_all_paths(rhs_val, &field_type, field_path, &mut comparison.added);
                }
                Some(lhs_val) => {
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

        // Check value type first to handle "sum types" like deduced schema
        // where atom has both scalar and map defined
        match value {
            // Handle lists
            Value::List(values) => {
                if let Some(ref list) = atom.list {
                    let mut new_values = Vec::new();
                    for (i, item) in values.iter().enumerate() {
                        let pe = if list.element_relationship == ElementRelationship::Associative {
                            if list.keys.is_empty() {
                                // Set semantics - use the value as the path element
                                PathElement::value(item.clone())
                            } else {
                                match self.list_item_to_key(item, list) {
                                    Ok(key) => PathElement::Key(key),
                                    Err(_) => PathElement::index(i as i32),
                                }
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
            }

            // Handle maps
            Value::Map(fields) => {
                if let Some(ref map) = atom.map {
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
                            // Keep the field even if value is null (field wasn't explicitly removed)
                            new_map.set(key.clone(), new_val);
                        }
                    }
                    // Return null if the map is now empty (all fields were explicitly removed)
                    if new_map.is_empty() {
                        return Value::Null;
                    }
                    return Value::Map(new_map);
                }
            }

            // Scalars and other values - nothing to remove
            _ => {}
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
                    if list.keys.is_empty() {
                        // Set semantics - use the value as the path element
                        PathElement::value(item.clone())
                    } else {
                        match self.list_item_to_key(item, list) {
                            Ok(key) => PathElement::Key(key),
                            Err(_) => PathElement::index(i as i32),
                        }
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
        // If rhs is null, it means "delete/clear" - use null
        if matches!(rhs, Value::Null) {
            return Value::Null;
        }

        // If lhs is null, use rhs
        if matches!(lhs, Value::Null) {
            return rhs.clone();
        }

        let atom = match self.schema.resolve(type_ref) {
            Some(atom) => atom,
            None => return rhs.clone(),
        };

        // Check value types first to handle "sum types" like deduced schema
        match (lhs, rhs) {
            (Value::Map(lhs_fields), Value::Map(rhs_fields)) => {
                if let Some(ref map) = atom.map {
                    if map.element_relationship == ElementRelationship::Atomic {
                        return rhs.clone();
                    }
                    return self.merge_maps(lhs_fields, rhs_fields, map);
                }
                // No map schema - replace with rhs
                rhs.clone()
            }
            (Value::List(lhs_items), Value::List(rhs_items)) => {
                if let Some(ref list) = atom.list {
                    if list.element_relationship == ElementRelationship::Atomic {
                        return rhs.clone();
                    }
                    return self.merge_lists(lhs_items, rhs_items, list);
                }
                // No list schema - replace with rhs
                rhs.clone()
            }
            _ => {
                // Scalar or type mismatch - RHS replaces LHS
                rhs.clone()
            }
        }
    }

    fn merge_lists(&self, lhs: &[Value], rhs: &[Value], list: &crate::schema::List) -> Value {
        if list.element_relationship == ElementRelationship::Associative {
            // Collect keys from both sides
            let mut rhs_key_set: std::collections::HashSet<FieldList> = std::collections::HashSet::new();
            let mut lhs_key_set: std::collections::HashSet<FieldList> = std::collections::HashSet::new();

            // For handling duplicates: map from key to list of values in LHS
            let mut lhs_by_key: std::collections::HashMap<FieldList, Vec<Value>> = std::collections::HashMap::new();

            for item in lhs {
                if let Ok(key) = self.list_item_to_key(item, list) {
                    lhs_key_set.insert(key.clone());
                    lhs_by_key.entry(key).or_insert_with(Vec::new).push(item.clone());
                }
            }

            for item in rhs {
                if let Ok(key) = self.list_item_to_key(item, list) {
                    rhs_key_set.insert(key.clone());
                }
            }

            // Check if this is a "pure set" (empty keys) or keyed list
            let is_set = list.keys.is_empty();

            // For sets: if RHS is a PROPER subset of LHS and LHS has no duplicates that RHS touches,
            // preserve LHS order. But if sets are equal, use RHS order.
            let rhs_subset_of_lhs = rhs_key_set.iter().all(|k| lhs_key_set.contains(k));
            let lhs_subset_of_rhs = lhs_key_set.iter().all(|k| rhs_key_set.contains(k));
            let lhs_has_rhs_duplicates = rhs_key_set.iter().any(|k| {
                lhs_by_key.get(k).map_or(false, |v| v.len() > 1)
            });
            let rhs_is_proper_subset = rhs_subset_of_lhs && !lhs_subset_of_rhs;

            if is_set && rhs_is_proper_subset && !lhs_has_rhs_duplicates {
                // For sets: RHS ⊆ LHS with no duplicates to resolve - preserve LHS
                Value::List(lhs.to_vec())
            } else {
                // General case: items only in LHS first, then RHS items in RHS order
                let mut result: Vec<Value> = Vec::new();

                // Add LHS items that are NOT in RHS (preserving order and duplicates)
                for item in lhs {
                    if let Ok(key) = self.list_item_to_key(item, list) {
                        if !rhs_key_set.contains(&key) {
                            result.push(item.clone());
                        }
                    }
                }

                // Add RHS items in RHS order
                // For keyed lists: merge with first LHS item if present
                // For sets: just use RHS item (deduplicates by only adding once)
                for item in rhs {
                    if let Ok(key) = self.list_item_to_key(item, list) {
                        // For keyed lists with actual keys, merge with LHS
                        if !is_set {
                            if let Some(lhs_items) = lhs_by_key.get(&key) {
                                if let Some(first_lhs) = lhs_items.first() {
                                    let merged = self.merge_values(first_lhs, item, &list.element_type);
                                    result.push(merged);
                                    continue;
                                }
                            }
                        }
                        // For sets or new items, just add
                        result.push(item.clone());
                    }
                }

                Value::List(result)
            }
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
