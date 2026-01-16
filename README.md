# structured-merge-diff

A Rust implementation of [structured-merge-diff](https://github.com/kubernetes-sigs/structured-merge-diff), providing structured merge and diff operations for Kubernetes Server-Side Apply (SSA).

## Overview

This library enables multi-manager field ownership tracking and conflict detection while performing merge operations on typed YAML/JSON objects. It is a port of the Go implementation used in Kubernetes.

## Features

- **Server-Side Apply (SSA)**: Full support for Kubernetes SSA merge semantics
- **Multi-manager ownership**: Track which manager owns which fields
- **Conflict detection**: Detect and report conflicts when multiple managers modify the same fields
- **Schema-based merging**: Type-aware merge operations using schema definitions
- **Version conversion**: Support for converting objects between API versions
- **Schema reconciliation**: Handle schema changes (granular to atomic, atomic to granular)

## Modules

| Module | Description |
|--------|-------------|
| `fieldpath` | Field path representation and management for tracking field ownership |
| `merge` | High-level multi-manager merge and apply operations |
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

## API Reference

### Updater

The main entry point for merge operations.

```rust
// Create with builder pattern
let updater = Updater::builder()
    .converter(Box::new(my_converter))      // Optional: version converter
    .ignore_filter(version, Box::new(filter)) // Optional: field filter
    .build();

// Apply a configuration (SSA apply)
updater.apply(live, config, version, managers, manager_name, force)?;

// Update an object (controller update)
updater.update(live, new_obj, version, managers, manager_name)?;
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
```

## Compatibility

This implementation is compatible with Go structured-merge-diff v6.3.0. All test cases from the Go implementation have been migrated and pass.

## Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test merge::merge_test

# Run with output
cargo test -- --nocapture
```

## License

Apache-2.0, matching the original Go implementation.
