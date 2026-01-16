//! Equality comparisons for schema types.

use super::elements::*;

impl PartialEq for Schema {
    fn eq(&self, other: &Self) -> bool {
        if self.types.len() != other.types.len() {
            return false;
        }
        self.types
            .iter()
            .zip(other.types.iter())
            .all(|(a, b)| a == b)
    }
}

impl Eq for Schema {}

impl PartialEq for TypeRef {
    fn eq(&self, other: &Self) -> bool {
        // Check if both have named_type or neither do
        match (&self.named_type, &other.named_type) {
            (Some(a), Some(b)) if a != b => return false,
            (Some(_), None) | (None, Some(_)) => return false,
            _ => {}
        }

        if self.element_relationship != other.element_relationship {
            return false;
        }

        self.inlined == other.inlined
    }
}

impl Eq for TypeRef {}

impl PartialEq for TypeDef {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.atom == other.atom
    }
}

impl Eq for TypeDef {}

impl PartialEq for Atom {
    fn eq(&self, other: &Self) -> bool {
        // Check if the presence/absence of each field matches
        if self.scalar.is_some() != other.scalar.is_some() {
            return false;
        }
        if self.list.is_some() != other.list.is_some() {
            return false;
        }
        if self.map.is_some() != other.map.is_some() {
            return false;
        }

        match (&self.scalar, &other.scalar) {
            (Some(a), Some(b)) if a != b => return false,
            _ => {}
        }

        match (&self.list, &other.list) {
            (Some(a), Some(b)) if a != b => return false,
            _ => {}
        }

        match (&self.map, &other.map) {
            (Some(a), Some(b)) if a != b => return false,
            _ => {}
        }

        true
    }
}

impl Eq for Atom {}

impl PartialEq for Map {
    fn eq(&self, other: &Self) -> bool {
        if self.element_type != other.element_type {
            return false;
        }
        if self.element_relationship != other.element_relationship {
            return false;
        }
        if self.fields.len() != other.fields.len() {
            return false;
        }
        for (a, b) in self.fields.iter().zip(other.fields.iter()) {
            if a != b {
                return false;
            }
        }
        if self.unions.len() != other.unions.len() {
            return false;
        }
        for (a, b) in self.unions.iter().zip(other.unions.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }
}

impl Eq for Map {}

impl PartialEq for Union {
    fn eq(&self, other: &Self) -> bool {
        match (&self.discriminator, &other.discriminator) {
            (Some(a), Some(b)) if a != b => return false,
            (Some(_), None) | (None, Some(_)) => return false,
            _ => {}
        }

        if self.deduce_invalid_discriminator != other.deduce_invalid_discriminator {
            return false;
        }

        if self.fields.len() != other.fields.len() {
            return false;
        }

        self.fields
            .iter()
            .zip(other.fields.iter())
            .all(|(a, b)| a == b)
    }
}

impl Eq for Union {}

impl PartialEq for UnionField {
    fn eq(&self, other: &Self) -> bool {
        self.field_name == other.field_name
            && self.discriminator_value == other.discriminator_value
    }
}

impl Eq for UnionField {}

impl PartialEq for StructField {
    fn eq(&self, other: &Self) -> bool {
        if self.name != other.name {
            return false;
        }
        if self.default != other.default {
            return false;
        }
        self.field_type == other.field_type
    }
}

impl Eq for StructField {}

impl PartialEq for List {
    fn eq(&self, other: &Self) -> bool {
        if self.element_type != other.element_type {
            return false;
        }
        if self.element_relationship != other.element_relationship {
            return false;
        }
        if self.keys.len() != other.keys.len() {
            return false;
        }
        self.keys.iter().zip(other.keys.iter()).all(|(a, b)| a == b)
    }
}

impl Eq for List {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_equality() {
        assert_eq!(Scalar::String, Scalar::String);
        assert_ne!(Scalar::String, Scalar::Numeric);
    }

    #[test]
    fn test_atom_equality() {
        let atom1 = Atom {
            scalar: Some(Scalar::String),
            ..Default::default()
        };
        let atom2 = Atom {
            scalar: Some(Scalar::String),
            ..Default::default()
        };
        let atom3 = Atom {
            scalar: Some(Scalar::Numeric),
            ..Default::default()
        };

        assert_eq!(atom1, atom2);
        assert_ne!(atom1, atom3);
    }

    #[test]
    fn test_type_ref_equality() {
        let tr1 = TypeRef {
            named_type: Some("foo".to_string()),
            ..Default::default()
        };
        let tr2 = TypeRef {
            named_type: Some("foo".to_string()),
            ..Default::default()
        };
        let tr3 = TypeRef {
            named_type: Some("bar".to_string()),
            ..Default::default()
        };
        let tr4 = TypeRef {
            inlined: Box::new(Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(tr1, tr2);
        assert_ne!(tr1, tr3);
        assert_ne!(tr1, tr4);
    }

    #[test]
    fn test_map_equality() {
        let map1 = Map::with_fields(vec![StructField {
            name: "foo".to_string(),
            ..Default::default()
        }]);
        let map2 = Map::with_fields(vec![StructField {
            name: "foo".to_string(),
            ..Default::default()
        }]);
        let map3 = Map::with_fields(vec![StructField {
            name: "bar".to_string(),
            ..Default::default()
        }]);

        assert_eq!(map1, map2);
        assert_ne!(map1, map3);
    }

    #[test]
    fn test_list_equality() {
        let list1 = List {
            element_relationship: ElementRelationship::Associative,
            keys: vec!["name".to_string()],
            ..Default::default()
        };
        let list2 = List {
            element_relationship: ElementRelationship::Associative,
            keys: vec!["name".to_string()],
            ..Default::default()
        };
        let list3 = List {
            element_relationship: ElementRelationship::Atomic,
            keys: vec![],
            ..Default::default()
        };

        assert_eq!(list1, list2);
        assert_ne!(list1, list3);
    }

    #[test]
    fn test_schema_equality() {
        let schema1 = Schema::with_types(vec![TypeDef {
            name: "foo".to_string(),
            atom: Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            },
        }]);
        let schema2 = Schema::with_types(vec![TypeDef {
            name: "foo".to_string(),
            atom: Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            },
        }]);
        let schema3 = Schema::with_types(vec![TypeDef {
            name: "bar".to_string(),
            atom: Atom {
                scalar: Some(Scalar::String),
                ..Default::default()
            },
        }]);

        assert_eq!(schema1, schema2);
        assert_ne!(schema1, schema3);
    }
}
