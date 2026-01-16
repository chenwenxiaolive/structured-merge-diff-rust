//! Updater for merge operations.

use crate::fieldpath::{APIVersion, ManagedFields, Set, VersionedSet};
use crate::typed::{Comparison, TypedValue, ValidationErrors};
use super::Conflicts;
use std::collections::HashMap;

/// Converter trait for version conversion.
pub trait Converter {
    /// Converts a TypedValue to a different API version.
    fn convert(&self, obj: &TypedValue, version: &APIVersion) -> Result<TypedValue, ConversionError>;

    /// Returns true if the error indicates a missing version.
    fn is_missing_version_error(&self, err: &ConversionError) -> bool;
}

/// ConversionError represents an error during version conversion.
#[derive(Debug, Clone)]
pub struct ConversionError {
    pub message: String,
    pub is_missing_version: bool,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConversionError {}

/// Filter trait for filtering fields.
pub trait Filter {
    /// Filters the given set.
    fn filter(&self, set: &Set) -> Set;
}

/// ExcludeSetFilter excludes a set of fields.
pub struct ExcludeSetFilter {
    excluded: Set,
}

impl ExcludeSetFilter {
    /// Creates a new ExcludeSetFilter.
    pub fn new(excluded: Set) -> Self {
        ExcludeSetFilter { excluded }
    }
}

impl Filter for ExcludeSetFilter {
    fn filter(&self, set: &Set) -> Set {
        set.difference(&self.excluded)
    }
}

/// UpdaterBuilder is a builder for creating an Updater.
#[derive(Default)]
pub struct UpdaterBuilder {
    converter: Option<Box<dyn Converter>>,
    ignore_filter: HashMap<APIVersion, Box<dyn Filter>>,
    ignored_fields: HashMap<APIVersion, Set>,
    return_input_on_noop: bool,
}

impl UpdaterBuilder {
    /// Creates a new UpdaterBuilder.
    pub fn new() -> Self {
        UpdaterBuilder::default()
    }

    /// Sets the converter.
    pub fn converter(mut self, converter: Box<dyn Converter>) -> Self {
        self.converter = Some(converter);
        self
    }

    /// Adds an ignore filter for a specific version.
    pub fn ignore_filter(mut self, version: APIVersion, filter: Box<dyn Filter>) -> Self {
        self.ignore_filter.insert(version, filter);
        self
    }

    /// Adds ignored fields for a specific version.
    pub fn ignored_fields(mut self, version: APIVersion, fields: Set) -> Self {
        self.ignored_fields.insert(version, fields);
        self
    }

    /// Sets whether to return input on no-op.
    pub fn return_input_on_noop(mut self, value: bool) -> Self {
        self.return_input_on_noop = value;
        self
    }

    /// Builds the Updater.
    pub fn build(self) -> Updater {
        Updater {
            converter: self.converter,
            ignore_filter: self.ignore_filter,
            ignored_fields: self.ignored_fields,
            return_input_on_noop: self.return_input_on_noop,
        }
    }
}

/// Updater is the main merge orchestrator.
pub struct Updater {
    converter: Option<Box<dyn Converter>>,
    ignore_filter: HashMap<APIVersion, Box<dyn Filter>>,
    ignored_fields: HashMap<APIVersion, Set>,
    pub return_input_on_noop: bool,
}

impl Updater {
    /// Creates a new UpdaterBuilder.
    pub fn builder() -> UpdaterBuilder {
        UpdaterBuilder::new()
    }

    /// Reconciles managed fields with any changes to the object's schema.
    ///
    /// Supports:
    /// - Changing types from atomic to granular
    /// - Changing types from granular to atomic
    fn reconcile_managed_fields_with_schema_changes(
        &self,
        live_object: &TypedValue,
        managers: &mut ManagedFields,
    ) -> Result<(), ApplyError> {
        use crate::typed::reconcile_field_set_with_schema;

        let mut updated_entries: Vec<(String, VersionedSet)> = Vec::new();

        for (manager, versioned_set) in managers.iter() {
            // Convert to the manager's version if needed
            let tv = if let Some(ref converter) = self.converter {
                match converter.convert(live_object, versioned_set.api_version()) {
                    Ok(v) => v,
                    Err(e) if converter.is_missing_version_error(&e) => {
                        // Okay to skip, obsolete versions will be deleted automatically anyway
                        continue;
                    }
                    Err(e) => return Err(ApplyError::ConversionError(e)),
                }
            } else {
                live_object.clone()
            };

            // Reconcile the field set with the schema
            match reconcile_field_set_with_schema(versioned_set.set(), &tv) {
                Ok(Some(reconciled)) => {
                    updated_entries.push((
                        manager.clone(),
                        VersionedSet::new(reconciled, versioned_set.api_version().clone(), versioned_set.applied()),
                    ));
                }
                Ok(None) => {
                    // No changes needed
                }
                Err(e) => {
                    return Err(ApplyError::ValidationError(ValidationErrors::from_error(
                        crate::typed::ValidationError::schema_error(&e),
                    )));
                }
            }
        }

        // Apply updates
        for (manager, vs) in updated_entries {
            managers.insert(manager, vs);
        }

        Ok(())
    }

    /// Internal update logic that computes conflicts and field changes.
    fn update_internal(
        &self,
        old_object: &TypedValue,
        new_object: &TypedValue,
        version: &APIVersion,
        managers: &mut ManagedFields,
        workflow: &str,
        force: bool,
    ) -> Result<Comparison, ApplyError> {
        // Compare old and new objects
        let compare = old_object.compare(new_object)
            .map_err(ApplyError::ValidationError)?;

        // Apply ignored fields filter if configured
        let filtered_compare = if let Some(fields) = self.ignored_fields.get(version) {
            let mut c = compare.clone();
            c.exclude_fields(fields);
            c
        } else if let Some(filter) = self.ignore_filter.get(version) {
            let mut c = compare.clone();
            let filter_set = Set::new();
            let filtered = filter.filter(&filter_set);
            c.filter_fields(&filtered);
            c
        } else {
            compare.clone()
        };

        // Track conflicts and removals
        let mut conflicts = Conflicts::new();
        let mut removed_by_manager: HashMap<String, Set> = HashMap::new();
        let mut obsolete_managers: Vec<String> = Vec::new();

        // Check each manager for conflicts
        for (manager, versioned_set) in managers.iter() {
            if manager == workflow {
                continue;
            }

            // Get the comparison at the manager's version
            let manager_compare = if versioned_set.api_version() == version {
                filtered_compare.clone()
            } else if let Some(ref converter) = self.converter {
                // Convert objects to manager's version for comparison
                let versioned_old = match converter.convert(old_object, versioned_set.api_version()) {
                    Ok(v) => v,
                    Err(e) if converter.is_missing_version_error(&e) => {
                        // Mark this manager as having an obsolete version
                        obsolete_managers.push(manager.clone());
                        continue;
                    },
                    Err(e) => return Err(ApplyError::ConversionError(e)),
                };
                let versioned_new = match converter.convert(new_object, versioned_set.api_version()) {
                    Ok(v) => v,
                    Err(e) if converter.is_missing_version_error(&e) => {
                        // Mark this manager as having an obsolete version
                        obsolete_managers.push(manager.clone());
                        continue;
                    },
                    Err(e) => return Err(ApplyError::ConversionError(e)),
                };

                versioned_old.compare(&versioned_new)
                    .map_err(ApplyError::ValidationError)?
            } else {
                filtered_compare.clone()
            };

            // Find conflicts: fields the manager owns that were modified or added
            let conflict_set = versioned_set.set()
                .intersection(&manager_compare.modified.union(&manager_compare.added));

            if !conflict_set.is_empty() {
                let mut paths = Vec::new();
                conflict_set.iterate(|path| paths.push(path.clone()));
                for path in paths {
                    conflicts.add(super::Conflict::new(manager.clone(), path));
                }
            }

            // Track removed fields
            if !manager_compare.removed.is_empty() {
                removed_by_manager.insert(manager.clone(), manager_compare.removed.clone());
            }
        }

        // Return conflicts if not forcing
        if !force && !conflicts.is_empty() {
            return Err(ApplyError::Conflicts(conflicts));
        }

        // Remove managers with obsolete versions
        for manager in obsolete_managers {
            managers.remove(&manager);
        }

        // Remove conflicting fields from other managers
        for conflict in conflicts.iter() {
            if let Some(vs) = managers.get(&conflict.manager) {
                let new_set = vs.set().difference(&conflicts.to_set());
                managers.insert(
                    conflict.manager.clone(),
                    VersionedSet::new(new_set, vs.api_version().clone(), vs.applied()),
                );
            }
        }

        // Remove fields that were removed from the object
        for (manager, removed_set) in removed_by_manager {
            if let Some(vs) = managers.get(&manager) {
                let new_set = vs.set().difference(&removed_set);
                managers.insert(
                    manager.clone(),
                    VersionedSet::new(new_set, vs.api_version().clone(), vs.applied()),
                );
            }
        }

        // Clean up empty manager entries
        managers.remove_empty();

        Ok(compare)
    }

    /// ExtractApply performs an extract-apply operation.
    ///
    /// This is like apply but additive - it doesn't remove fields that the manager
    /// previously owned but are not in the current config. It adds the new fields
    /// to the manager's ownership while keeping the old ones.
    pub fn extract_apply(
        &self,
        live_obj: &TypedValue,
        config_obj: &TypedValue,
        version: &APIVersion,
        managers: &mut ManagedFields,
        manager: &str,
        force: bool,
    ) -> Result<TypedValue, ApplyError> {
        // Merge config into live object
        let new_object = live_obj.merge(config_obj)
            .map_err(ApplyError::ValidationError)?;

        // Get the field set from the config
        let config_set = config_obj.to_field_set()
            .map_err(ApplyError::ValidationError)?;

        // Apply ignored fields filter
        let filtered_set = if let Some(fields) = self.ignored_fields.get(version) {
            config_set.recursive_difference(fields)
        } else if let Some(filter) = self.ignore_filter.get(version) {
            filter.filter(&config_set)
        } else {
            config_set
        };

        // Get the previous set for this manager (for union, not pruning)
        let last_set = managers.get(manager).map(|vs| vs.set().clone());

        // For extract_apply, we UNION with the previous set instead of replacing
        let new_manager_set = if let Some(ls) = last_set {
            ls.union(&filtered_set)
        } else {
            filtered_set
        };

        // Update manager's field set
        managers.insert(
            manager.to_string(),
            VersionedSet::new(new_manager_set, version.clone(), false),
        );

        // Run update to check for conflicts with other managers
        self.update_internal(live_obj, &new_object, version, managers, manager, force)?;

        Ok(new_object)
    }

    /// Apply performs an apply operation.
    ///
    /// This merges the config object into the live object, tracking field ownership.
    pub fn apply(
        &self,
        live_obj: &TypedValue,
        config_obj: &TypedValue,
        version: &APIVersion,
        managers: &mut ManagedFields,
        manager: &str,
        force: bool,
    ) -> Result<TypedValue, ApplyError> {
        // Reconcile managed fields with any schema changes
        self.reconcile_managed_fields_with_schema_changes(live_obj, managers)?;

        // Merge config into live object
        let new_object = live_obj.merge(config_obj)
            .map_err(ApplyError::ValidationError)?;

        // Get the field set from the config
        let config_set = config_obj.to_field_set()
            .map_err(ApplyError::ValidationError)?;

        // Apply ignored fields filter
        let filtered_set = if let Some(fields) = self.ignored_fields.get(version) {
            config_set.recursive_difference(fields)
        } else if let Some(filter) = self.ignore_filter.get(version) {
            filter.filter(&config_set)
        } else {
            config_set
        };

        // Store the previous set for this manager (for pruning and rollback)
        let last_set = managers.get(manager).cloned();

        // Check if the previous version is obsolete (can't be converted)
        let prev_version_obsolete = if let Some(ref ls) = last_set {
            if ls.api_version() == version {
                false
            } else if let Some(ref converter) = self.converter {
                // Try to convert to the old version to see if it's still valid
                match converter.convert(live_obj, ls.api_version()) {
                    Ok(_) => false,
                    Err(e) if converter.is_missing_version_error(&e) => true,
                    Err(_) => false, // Other errors don't indicate obsolete version
                }
            } else {
                false
            }
        } else {
            false
        };

        // Temporarily update manager's field set (needed for pruning logic)
        managers.insert(
            manager.to_string(),
            VersionedSet::new(filtered_set.clone(), version.clone(), true),
        );

        // Prune fields that were removed from the config
        // Skip pruning if the previous version is obsolete (we can't determine what was previously owned)
        let pruned_object = if !prev_version_obsolete {
            if let Some(ref ls) = last_set {
                if !ls.set().is_empty() {
                    let removed_from_config = ls.set().difference(&filtered_set);
                    if !removed_from_config.is_empty() {
                        // Remove fields that this manager owned but no longer does
                        // unless another manager owns them
                        let mut to_remove = Set::new();
                        removed_from_config.iterate(|path| {
                            let mut owned_by_other = false;
                            for (other_manager, other_vs) in managers.iter() {
                                if other_manager != manager && other_vs.set().has(path) {
                                    owned_by_other = true;
                                    break;
                                }
                            }
                            if !owned_by_other {
                                to_remove.insert(path);
                            }
                        });
                        new_object.remove_items(&to_remove)
                    } else {
                        new_object
                    }
                } else {
                    new_object
                }
            } else {
                new_object
            }
        } else {
            new_object
        };

        // Run update to check for conflicts with other managers
        let result = self.update_internal(live_obj, &pruned_object, version, managers, manager, force);

        // If there's a conflict, roll back the manager entry
        if result.is_err() {
            // Restore the previous state
            if let Some(ls) = last_set {
                managers.insert(manager.to_string(), ls);
            } else {
                managers.remove(manager);
            }
            return result.map(|_| pruned_object);
        }

        // Check for no-op
        if !self.return_input_on_noop && live_obj.value() == pruned_object.value() {
            // Return the pruned object anyway since we need to track managers
        }

        Ok(pruned_object)
    }

    /// Update performs an update operation.
    ///
    /// This updates the live object with the new object, tracking field ownership.
    pub fn update(
        &self,
        live_obj: &TypedValue,
        new_obj: &TypedValue,
        version: &APIVersion,
        managers: &mut ManagedFields,
        manager: &str,
    ) -> Result<TypedValue, UpdateError> {
        // Reconcile managed fields with any schema changes
        self.reconcile_managed_fields_with_schema_changes(live_obj, managers)
            .map_err(|e| match e {
                ApplyError::Conflicts(c) => UpdateError::Conflicts(c),
                ApplyError::ConversionError(e) => UpdateError::ConversionError(e),
                ApplyError::ValidationError(e) => UpdateError::ValidationError(e),
                ApplyError::NotImplemented => UpdateError::NotImplemented,
            })?;

        // Run update with force=true (updates don't conflict)
        let compare = self.update_internal(live_obj, new_obj, version, managers, manager, true)
            .map_err(|e| match e {
                ApplyError::Conflicts(c) => UpdateError::Conflicts(c),
                ApplyError::ConversionError(e) => UpdateError::ConversionError(e),
                ApplyError::ValidationError(e) => UpdateError::ValidationError(e),
                ApplyError::NotImplemented => UpdateError::NotImplemented,
            })?;

        // Get or create manager entry
        let current_set = managers.get(manager)
            .map(|vs| vs.set().clone())
            .unwrap_or_default();

        // Update manager's field set:
        // - Remove fields that were removed
        // - Add fields that were modified or added
        let new_set = current_set
            .difference(&compare.removed)
            .union(&compare.modified)
            .union(&compare.added);

        // Apply ignored fields filter
        let filtered_set = if let Some(fields) = self.ignored_fields.get(version) {
            new_set.recursive_difference(fields)
        } else if let Some(filter) = self.ignore_filter.get(version) {
            filter.filter(&new_set)
        } else {
            new_set
        };

        // Update manager entry
        if filtered_set.is_empty() {
            managers.remove(manager);
        } else {
            managers.insert(
                manager.to_string(),
                VersionedSet::new(filtered_set, version.clone(), false),
            );
        }

        Ok(new_obj.clone())
    }
}

/// ApplyError represents an error during apply.
#[derive(Debug, Clone)]
pub enum ApplyError {
    Conflicts(Conflicts),
    ConversionError(ConversionError),
    ValidationError(ValidationErrors),
    NotImplemented,
}

impl std::fmt::Display for ApplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApplyError::Conflicts(c) => write!(f, "conflicts: {}", c),
            ApplyError::ConversionError(e) => write!(f, "conversion error: {}", e),
            ApplyError::ValidationError(e) => write!(f, "validation error: {}", e),
            ApplyError::NotImplemented => write!(f, "not implemented"),
        }
    }
}

impl std::error::Error for ApplyError {}

/// UpdateError represents an error during update.
#[derive(Debug, Clone)]
pub enum UpdateError {
    Conflicts(Conflicts),
    ConversionError(ConversionError),
    ValidationError(ValidationErrors),
    NotImplemented,
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Conflicts(c) => write!(f, "conflicts: {}", c),
            UpdateError::ConversionError(e) => write!(f, "conversion error: {}", e),
            UpdateError::ValidationError(e) => write!(f, "validation error: {}", e),
            UpdateError::NotImplemented => write!(f, "not implemented"),
        }
    }
}

impl std::error::Error for UpdateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Atom, Map as SchemaMap, Schema, TypeDef, TypeRef, Scalar};
    use crate::value::{Map, Value};

    fn create_test_schema() -> Schema {
        Schema::with_types(vec![TypeDef {
            name: "object".to_string(),
            atom: Atom {
                map: Some(SchemaMap::with_element_type(TypeRef {
                    named_type: Some("scalar".to_string()),
                    ..Default::default()
                })),
                ..Default::default()
            },
        }, TypeDef {
            name: "scalar".to_string(),
            atom: Atom {
                scalar: Some(Scalar::Untyped),
                ..Default::default()
            },
        }])
    }

    #[test]
    fn test_updater_builder() {
        let updater = Updater::builder()
            .return_input_on_noop(true)
            .build();

        assert!(updater.return_input_on_noop);
    }

    #[test]
    fn test_update_simple() {
        let updater = Updater::builder().build();
        let schema = create_test_schema();
        let type_ref = TypeRef {
            named_type: Some("object".to_string()),
            ..Default::default()
        };

        let mut live_map = Map::new();
        live_map.set("a".to_string(), Value::String("1".into()));
        let live_obj = TypedValue::new(Value::Map(live_map), schema.clone(), type_ref.clone());

        let mut new_map = Map::new();
        new_map.set("a".to_string(), Value::String("2".into()));
        new_map.set("b".to_string(), Value::String("3".into()));
        let new_obj = TypedValue::new(Value::Map(new_map), schema.clone(), type_ref.clone());

        let version = APIVersion::new("v1");
        let mut managers = ManagedFields::new();

        let result = updater.update(&live_obj, &new_obj, &version, &mut managers, "manager1");
        assert!(result.is_ok());

        // Manager1 should own the modified and added fields
        let manager_set = managers.get("manager1").unwrap();
        assert!(manager_set.set().has(&crate::fieldpath::Path::from_elements(vec![
            crate::fieldpath::PathElement::field_name("a")
        ])));
        assert!(manager_set.set().has(&crate::fieldpath::Path::from_elements(vec![
            crate::fieldpath::PathElement::field_name("b")
        ])));
    }

    #[test]
    fn test_apply_simple() {
        let updater = Updater::builder().build();
        let schema = create_test_schema();
        let type_ref = TypeRef {
            named_type: Some("object".to_string()),
            ..Default::default()
        };

        let mut live_map = Map::new();
        live_map.set("a".to_string(), Value::String("1".into()));
        let live_obj = TypedValue::new(Value::Map(live_map), schema.clone(), type_ref.clone());

        let mut config_map = Map::new();
        config_map.set("b".to_string(), Value::String("2".into()));
        let config_obj = TypedValue::new(Value::Map(config_map), schema.clone(), type_ref.clone());

        let version = APIVersion::new("v1");
        let mut managers = ManagedFields::new();

        let result = updater.apply(&live_obj, &config_obj, &version, &mut managers, "manager1", false);
        assert!(result.is_ok());

        let merged = result.unwrap();
        // Merged should have both a and b
        if let Value::Map(m) = merged.value() {
            assert!(m.get("a").is_some());
            assert!(m.get("b").is_some());
        } else {
            panic!("Expected map value");
        }
    }
}
