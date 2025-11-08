// flexicon/src/adaptive/namedmap.rs
//
// Copyright (c) 2025 Arcella Team
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE>
// or the MIT license <LICENSE-MIT>, at your option.
// This file may not be copied, modified, or distributed
// except according to those terms.

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

/// A trait for types that can be constructed from a name string.
///
/// This trait enables `NamedMap<T>` to support **dual-format input**:
/// when deserializing from a simple list like `["iface1", "iface2"]`,
/// each name is turned into a meaningful default value via `from_name`.
///
/// # Example
///
/// ```rust
/// use flexicon::adaptive::FromName;
///
/// #[derive(Clone)]
/// struct Capability {
///     name: String,
///     version: String,
/// }
///
/// impl FromName for Capability {
///     fn from_name(name: &str) -> Self {
///         Self {
///             name: name.to_string(),
///             version: "latest".to_string(),
///         }
///     }
/// }
/// ```
pub trait FromName: Clone {
    /// Construct a value from its name.
    ///
    /// This method should never fail. Even if the name is malformed,
    /// return a sensible fallback (e.g., with `version: "unknown"`).
    /// Validation, if needed, should happen at a higher layer.
    fn from_name(name: &str) -> Self;
}

/// A map of named items that supports **adaptive deserialization**:
///
/// - **Human-friendly format**: `["a", "b"]`  
///   → each name is converted to a placeholder using `FromName`.
/// - **Machine-friendly format**: `{ "a": {...}, "b": {...} }`  
///   → full structured values are parsed as-is.
///
/// This enables configurations that are **easy to write** and **rich to process**.
///
/// # Key properties
///
/// - Transparently wraps `HashMap<String, T>` (`Deref`/`DerefMut` implemented).
/// - Always serializes to the detailed (object) form for canonical output.
/// - Supports any `serde` format (TOML, JSON, YAML, etc.) when the `serde` feature is enabled.
/// - Provides JSON-specific utilities (e.g., `to_json_string`) when `serde_json` is enabled.
///
/// # Example (with serde)
///
/// ```rust
/// # use serde::{Deserialize, Serialize};
/// # #[derive(Clone, Serialize, Deserialize)]
/// # struct Interface { version: String }
/// # impl flexicon::adaptive::FromName for Interface {
/// #     fn from_name(name: &str) -> Self { Self { version: "latest".into() } }
/// # }
/// use flexicon::adaptive::NamedMap;
///
/// # #[cfg(feature = "serde_json")]
/// # {
/// // Simple config (user-authored)
/// let simple: NamedMap<Interface> = serde_json::from_str(r#"["logger", "http"]"#)?;
///
/// // Detailed config (tool-generated)
/// let detailed: NamedMap<Interface> = serde_json::from_str(r#"
/// {
///   "logger": { "version": "1.0" },
///   "http": { "version": "0.2" }
/// }
/// "#)?;
/// # Ok::<(), serde_json::Error>(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedMap<T>(HashMap<String, T>);

impl<T> NamedMap<T> {
    /// Creates an empty `NamedMap`.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Inserts a key-value pair into the map.
    pub fn insert(&mut self, key: String, value: T) {
        self.0.insert(key, value);
    }

    /// Returns `true` if the map contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Consumes the map and returns the inner `HashMap`.
    pub fn into_inner(self) -> HashMap<String, T> {
        self.0
    }

    /// Returns a reference to the inner map.
    pub fn as_inner(&self) -> &HashMap<String, T> {
        &self.0
    }

    /// Returns a mutable reference to the inner map.
    ///
    /// ⚠️ **Warning**: Direct mutation bypasses any future validation or invariants
    /// that `NamedMap` might enforce (e.g., key normalization, version parsing).
    /// Prefer using `insert` or higher-level APIs when possible.
    pub fn as_inner_mut(&mut self) -> &mut HashMap<String, T> {
        &mut self.0
    }
}

impl<T> Default for NamedMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Make `NamedMap<T>` behave like a `HashMap` for seamless use.
impl<T> Deref for NamedMap<T> {
    type Target = HashMap<String, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for NamedMap<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Allow construction from a simple list of names (e.g., TOML: `interfaces = ["a", "b"]`)
impl<T: FromName + Clone> From<Vec<String>> for NamedMap<T> {
    fn from(list: Vec<String>) -> Self {
        let mut map = HashMap::new();
        for name in list {
            map.insert(name.clone(), T::from_name(&name));
        }
        NamedMap(map)
    }
}

// === SERDE INTEGRATION (format-agnostic) ===

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{
        de::{DeserializeOwned, Deserializer, MapAccess, SeqAccess, Visitor},
        ser::Serializer,
        Deserialize, Serialize,
    };
    use std::fmt;
    use std::marker::PhantomData;

    /// Visitor that handles both array-of-strings and object formats.
    #[derive(Debug)]
    struct NamedMapVisitor<T> {
        _phantom: PhantomData<T>,
    }

    impl<'de, T> Visitor<'de> for NamedMapVisitor<T>
    where
        T: DeserializeOwned + FromName + Clone,
    {
        type Value = NamedMap<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(formatter, "either a map (e.g., {{ \"a\": {{...}} }}) or a sequence of strings (e.g., [\"a\", \"b\"])")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut map = HashMap::new();
            while let Some(name) = seq.next_element::<String>()? {
                map.insert(name.clone(), T::from_name(&name));
            }
            Ok(NamedMap(map))
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            // Delegate to standard map deserialization
            let inner = Deserialize::deserialize(serde::de::value::MapAccessDeserializer::new(map))?;
            Ok(NamedMap(inner))
        }
    }

    impl<T> Serialize for NamedMap<T>
    where
        T: Serialize,
    {
        /// Serializes as a JSON/TOML/YAML object (never as an array).
        /// This ensures canonical, lossless output.
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.0.serialize(serializer)
        }
    }

    impl<'de, T> Deserialize<'de> for NamedMap<T>
    where
        T: DeserializeOwned + FromName + Clone,
    {
        /// Deserializes from either:
        /// - An object (detailed form)
        /// - An array of strings (simple form)
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(NamedMapVisitor {
                _phantom: PhantomData,
            })
        }
    }
}

// === JSON-SPECIFIC CONVENIENCE METHODS ===

#[cfg(feature = "serde_json")]
impl<T> NamedMap<T>
where
    T: serde::Serialize,
{
    /// Serialize this map to a `serde_json::Value`.
    ///
    /// Equivalent to `serde_json::to_value(&map)`, but with explicit intent.
    pub fn to_json_value(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(&self.0)
    }

    /// Serialize this map to a compact JSON string.
    pub fn to_json_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.0)
    }
}

#[cfg(feature = "serde_json")]
impl<T> NamedMap<T>
where
    T: for<'de> serde::Deserialize<'de> + FromName + Clone,
{
    /// Parse a `NamedMap` from a `serde_json::Value`.
    ///
    /// Supports the same dual formats as the general `Deserialize` impl:
    /// - Object: `{ "a": {...} }`
    /// - Array: `["a", "b"]`
    pub fn from_json_value(value: serde_json::Value) -> serde_json::Result<Self> {
        match &value {
            serde_json::Value::Object(_) => {
                let inner = serde_json::from_value(value)?;
                Ok(NamedMap(inner))
            }
            serde_json::Value::Array(arr) => {
                let mut map = HashMap::new();
                for item in arr {
                    let s = item
                        .as_str()
                        .ok_or_else(|| serde_json::Error::custom("array items must be strings"))?;
                    map.insert(s.to_string(), T::from_name(s));
                }
                Ok(NamedMap(map))
            }
            _ => Err(serde_json::Error::custom(
                "NamedMap must be an object or array of strings",
            )),
        }
    }

    /// Parse a `NamedMap` from a JSON string.
    ///
    /// Useful for config loading or API parsing.
    pub fn from_json_str(s: &str) -> serde_json::Result<Self> {
        let value: serde_json::Value = serde_json::from_str(s)?;
        Self::from_json_value(value)
    }
}

// === TESTS ===

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "serde")]
    use serde::{Deserialize, Serialize};

    #[cfg(feature = "serde")]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct TestItem {
        value: String,
        optional: bool,
    }

    #[cfg(feature = "serde")]
    impl FromName for TestItem {
        fn from_name(name: &str) -> Self {
            Self {
                value: format!("from_name({})", name),
                optional: false,
            }
        }
    }

    #[test]
    fn test_new_is_empty() {
        let map: NamedMap<()> = NamedMap::new();
        assert!(map.is_empty());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_insert_and_get() {
        let mut map = NamedMap::new();
        map.insert("key1".to_string(), TestItem {
            value: "value1".to_string(),
            optional: true,
        });
        assert_eq!(map["key1"].value, "value1");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_deref_transparency() {
        let mut map = NamedMap::new();
        map.insert("test".to_string(), TestItem {
            value: "ok".to_string(),
            optional: false,
        });
        assert_eq!(map.get("test").unwrap().value, "ok");
    }

    #[cfg(all(feature = "serde", feature = "serde_json"))]
    #[test]
    fn test_json_roundtrip_array() {
        let original: NamedMap<TestItem> = NamedMap::from(vec!["a".to_string(), "b".to_string()]);
        let json_str = original.to_json_string().unwrap();
        let restored = NamedMap::from_json_str(&json_str).unwrap();
        assert_eq!(original, restored);
    }

    #[cfg(all(feature = "serde", feature = "serde_json"))]
    #[test]
    fn test_json_roundtrip_object() {
        let mut original = NamedMap::new();
        original.insert("handler".to_string(), TestItem {
            value: "http".to_string(),
            optional: true,
        });
        let json_str = original.to_json_string().unwrap();
        let restored = NamedMap::from_json_str(&json_str).unwrap();
        assert_eq!(original, restored);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_toml_compatibility() {
        // This test ensures the main serde path does NOT depend on serde_json.
        // It verifies compatibility with TOML, commonly used in Arcella and walmq manifests.
        let toml_str = r#"
            logger = { value = "file", optional = true }
            metrics = { value = "prom", optional = false }
        "#;
        let _map: NamedMap<TestItem> = toml::from_str(toml_str).unwrap();
    }

    #[test]
    fn test_default_impl() {
        let map: NamedMap<()> = NamedMap::default();
        assert!(map.is_empty());
    }
}
