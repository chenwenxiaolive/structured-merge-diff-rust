# structured-merge-diff

A Rust implementation of [structured-merge-diff](https://github.com/kubernetes-sigs/structured-merge-diff), providing structured merge and diff operations for Kubernetes Server-Side Apply (SSA).

## Overview

This library enables multi-manager field ownership tracking and conflict detection while performing merge operations on typed YAML/JSON objects. It is a port of the Go implementation used in Kubernetes.

## Features

- **Server-Side Apply (SSA)**: Full support for Kubernetes SSA merge semantics
- **Multi-manager ownership**: Track which manager owns which fields
- **Conflict detection**: Detect and report conflicts when multiple managers modify the same fields
- **Schema-based merging**: Type-aware merge operations using schema definitions
- **OpenAPI support**: Parse OpenAPI v2 (Swagger) and v3 documents and convert to SMD schema
- **Version conversion**: Support for converting objects between API versions
- **Schema reconciliation**: Handle schema changes (granular to atomic, atomic to granular)
- **Field path serialization**: Compatible serialization format with Go implementation

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
structured-merge-diff = "6.3.0"
```

## Modules

| Module | Description |
|--------|-------------|
| `fieldpath` | Field path representation, serialization, and management for tracking field ownership |
| `merge` | High-level multi-manager merge and apply operations with conflict detection |
| `openapi` | Parse OpenAPI v2/v3 documents and convert to SMD schema |
| `schema` | Type schema definition language for structured merge operations |
| `typed` | Operations on Values with specific schemas (validation, comparison, merging) |
| `value` | In-memory representation of YAML/JSON objects |

## Usage

### Basic Apply Operation

```rust
use structured_merge_diff::{
    Updater, ManagedFields, APIVersion,
    typed::deduced_parseable_type,
};

// Create an updater
let updater = Updater::builder().build();

// Parse objects using deduced schema
let pt = deduced_parseable_type();
let live = pt.from_yaml("{}").unwrap();
let config = pt.from_yaml(r#"{"name": "test", "value": 42}"#).unwrap();

// Track managed fields
let mut managers = ManagedFields::new();

// Apply configuration
let result = updater.apply(
    &live,
    &config,
    &APIVersion::new("v1"),
    &mut managers,
    "my-manager",
    false,  // force
).unwrap();
```

### Using a Schema

```rust
use structured_merge_diff::typed::Parser;

let schema_yaml = r#"
types:
  - name: myType
    map:
      fields:
        - name: name
          type:
            scalar: string
        - name: replicas
          type:
            scalar: numeric
"#;

let parser = Parser::new(schema_yaml).unwrap();
let pt = parser.type_by_name("myType");
let obj = pt.from_yaml(r#"{"name": "test", "replicas": 3}"#).unwrap();
```

### Conflict Detection

```rust
use structured_merge_diff::{Updater, ManagedFields, APIVersion, ApplyError};

let updater = Updater::builder().build();
let mut managers = ManagedFields::new();

// First manager applies
let result1 = updater.apply(&live, &config1, &version, &mut managers, "manager1", false);

// Second manager tries to apply conflicting changes
let result2 = updater.apply(&result1.unwrap(), &config2, &version, &mut managers, "manager2", false);

match result2 {
    Err(ApplyError::Conflicts(conflicts)) => {
        println!("Conflicts detected:\n{}", conflicts);
    }
    Ok(_) => println!("No conflicts"),
    Err(e) => println!("Error: {:?}", e),
}
```

### Extract and Apply

```rust
use structured_merge_diff::{Updater, ManagedFields, APIVersion};

let updater = Updater::builder().build();
let mut managers = ManagedFields::new();

// Extract fields owned by a manager from live object, then apply new config
let result = updater.extract_apply(
    &live,
    &config,
    &APIVersion::new("v1"),
    &mut managers,
    "my-manager",
    false,  // force
).unwrap();
```

### Using OpenAPI Schema

Convert OpenAPI v2 (Swagger) or v3 documents to SMD schema format:

```rust
use structured_merge_diff::openapi::{OpenAPIDocument, convert_openapi_to_schema};

// Parse from JSON
let json = r##"{
    "swagger": "2.0",
    "info": {"title": "My API", "version": "1.0"},
    "definitions": {
        "Pod": {
            "type": "object",
            "properties": {
                "metadata": {"$ref": "#/definitions/ObjectMeta"},
                "spec": {"$ref": "#/definitions/PodSpec"}
            }
        },
        "PodSpec": {
            "type": "object",
            "properties": {
                "containers": {
                    "type": "array",
                    "items": {"$ref": "#/definitions/Container"},
                    "x-kubernetes-list-type": "map",
                    "x-kubernetes-list-map-keys": ["name"]
                }
            }
        },
        "Container": {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "image": {"type": "string"}
            }
        },
        "ObjectMeta": {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "labels": {
                    "type": "object",
                    "additionalProperties": {"type": "string"},
                    "x-kubernetes-map-type": "granular"
                }
            }
        }
    }
}"##;

let doc = OpenAPIDocument::from_json(json).unwrap();
let result = convert_openapi_to_schema(&doc);

// Check for conversion errors
for err in &result.errors {
    eprintln!("Warning: {}", err);
}

// Use the converted schema
let schema = result.schema;
println!("Converted {} types", schema.types.len());
```

OpenAPI v3 documents are also supported:

```rust
let json = r#"{
    "openapi": "3.0.0",
    "info": {"title": "My API", "version": "1.0"},
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
```

The converter supports all Kubernetes OpenAPI extensions:
- `x-kubernetes-list-type`: atomic, set, map
- `x-kubernetes-list-map-keys`: keys for map-type lists
- `x-kubernetes-map-type`: atomic, granular
- `x-kubernetes-preserve-unknown-fields`: preserve unknown fields
- `x-kubernetes-int-or-string`: field can be int or string
- `x-kubernetes-embedded-resource`: embedded resource
- `x-kubernetes-unions`: union discriminators

## API Reference

### Updater

The main entry point for merge operations.

```rust
// Create with builder pattern
let updater = Updater::builder()
    .converter(Box::new(my_converter))        // Optional: version converter
    .ignore_filter(version, Box::new(filter)) // Optional: field filter
    .ignored_fields(version, fields)          // Optional: ignored field set
    .build();

// Apply a configuration (SSA apply)
updater.apply(live, config, version, managers, manager_name, force)?;

// Update an object (controller update)
updater.update(live, new_obj, version, managers, manager_name)?;

// Extract and apply
updater.extract_apply(live, config, version, managers, manager_name, force)?;
```

### TypedValue

Represents a value with an associated schema.

```rust
// Validation
typed_value.validate(&[])?;

// Convert to field set
let fields = typed_value.to_field_set()?;

// Compare two values
let comparison = typed_value.compare(&other)?;

// Merge two values
let merged = typed_value.merge(&other)?;

// Remove specific fields
let pruned = typed_value.remove_items(&fields_to_remove);

// Extract specific fields
let extracted = typed_value.extract_items(&fields_to_extract);

// Create empty value with same schema
let empty = typed_value.empty();
```

### ManagedFields

Tracks field ownership per manager.

```rust
let mut managers = ManagedFields::new();

// Insert a manager's fields
managers.insert("manager1", VersionedSet::new(field_set, version, applied));

// Get a manager's fields
if let Some(vs) = managers.get("manager1") {
    println!("Manager owns: {:?}", vs.set());
}

// Iterate over all managers
for (name, vs) in managers.iter() {
    println!("{}: {:?}", name, vs.set());
}

// Compute difference between two ManagedFields
let diff = managers1.difference(&managers2);
```

### Set (Field Set)

Represents a set of field paths.

```rust
let mut set = Set::new();

// Insert a path
set.insert(&path);

// Check if path exists
if set.has(&path) { ... }

// Set operations
let union = set1.union(&set2);
let intersection = set1.intersection(&set2);
let difference = set1.difference(&set2);

// Iterate over all paths
set.iterate(|path| {
    println!("{}", path);
});
```

## Compatibility

This implementation is compatible with Go structured-merge-diff v6.3.0. All test cases from the Go implementation have been migrated and pass.

### Migrated Test Coverage

| Go Test File | Tests |
|--------------|-------|
| conflict_test.go | 3 |
| deduced_test.go | 9 |
| default_keys_test.go | 8 |
| duplicates_test.go | 7 |
| extract_apply_test.go | 14 |
| field_level_overrides_test.go | 4 |
| ignore_test.go | 6 |
| key_test.go | 2 |
| leaf_test.go | 6 |
| multiple_appliers_test.go | 11 |
| nested_test.go | 14 |
| obsolete_versions_test.go | 3 |
| openapi (new) | 8 |
| preserve_unknown_test.go | 1 |
| schema_change_test.go | 4 |
| set_test.go | 10 |
| **Total** | **285 tests** |

## Development

### Building

```bash
cargo build
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test merge::merge_test

# Run with output
cargo test -- --nocapture
```

### Linting

```bash
cargo clippy
```

## License

Apache-2.0, matching the original Go implementation.
