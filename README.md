# flexicon

**Configuration should adapt to the user — not the other way around.**

`flexicon` is a minimal, embeddable toolkit for building **adaptive data structures** that seamlessly bridge human-authored simplicity and machine-ready expressiveness.

[![crates.io](https://img.shields.io/crates/v/flexicon.svg)](https://crates.io/crates/flexicon)  
[![docs.rs](https://img.shields.io/docsrs/flexicon)](https://docs.rs/flexicon)  
[![License: Apache-2.0/MIT](https://img.shields.io/badge/license-Apache%202.0%20%7C%20MIT-blue)](https://github.com/ArcellaTeam/flexicon)

Write your config as a list of names.  
Refine it later into a full spec.  
Your tools understand both — without compromise.

---

## 🌱 Core Philosophy: Adaptive Data Structures

At the heart of `flexicon` lies a simple idea:

> **Users deserve the easiest possible interface.  
> Systems deserve the richest possible specification.  
> There’s no need to choose.**

We enable types that support **dual representations**:

- **Simple**: for humans (`["logger", "http"]`)
- **Detailed**: for machines (`{ "logger": { "level": "debug" }, "http": { "port": 8080 } }`)

This is formalized via the `AdaptiveFormat` trait:

```rust
pub trait AdaptiveFormat: Serialize + Deserialize<'static> {
    type SimpleFormat;
    type DetailedFormat;
    
    fn to_simple(&self) -> Self::SimpleFormat;
    fn from_simple(simple: Self::SimpleFormat) -> Self;
    fn to_detailed(&self) -> Self::DetailedFormat;
}
```

Types implementing this trait can fluidly move between user-friendly and system-optimized forms — automatically.

---

## 🧱 Core Components

### `adaptive::NamedMap<T>`

A zero-overhead wrapper around `HashMap<String, T>` that **accepts both formats out of the box**:

- As an **array of strings** → creates placeholder values using `T::from_name()`
- As an **object** → parses full structured values

```rust
use flexicon::adaptive::NamedMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
struct Interface {
    version: String,
    optional: bool,
}

impl flexicon::FromName for Interface {
    fn from_name(name: &str) -> Self {
        Self {
            version: "latest".into(),
            optional: false,
        }
    }
}

// Human-friendly config
let simple: NamedMap<Interface> = serde_json::from_str(r#"["wasi:cli/stdio", "my:logger"]"#)?;

// Machine-optimized spec
let detailed: NamedMap<Interface> = serde_json::from_str(r#"
{
  "wasi:cli/stdio": { "version": "0.2", "optional": false },
  "my:logger": { "version": "1.0", "optional": true }
}
"#)?;
```

`NamedMap<T>` is the first building block in the `flexicon` ecosystem — designed to be **embeddable**, **serde-optional**, and **dependency-free** beyond core traits.

---

## 🛠️ Modules Overview

| Module | Purpose |
|-------|--------|
| `adaptive` | Core adaptive containers (`NamedMap<T>`, future `VersionedSet`, etc.) |
| `humanize` | Traits for human-readable rendering (`HumanReadable`, `MachineOptimized`) |
| `format` | Serialization helpers for dual-format input/output (`DualFormat<T>`, adaptive serializers) |

> **Note**: As of v0.1, only `adaptive::NamedMap<T>` is implemented. The rest define the architecture for future expansion.

---

## 📦 Why `flexicon`?

- ✅ **User-first configs**: let users write less, express more
- ✅ **No runtime cost**: zero-copy where possible, no hidden allocations
- ✅ **No framework lock-in**: pure data structures, no runtime or macros
- ✅ **Embeddable**: < 500 lines of core logic, no heavy dependencies

Perfect for:
- Component manifests (e.g., in [Arcella](https://github.com/ArcellaTeam/arcella))
- Edge/IoT application descriptors
- CLI tools with layered config
- Any system where **developer experience** and **system reliability** must coexist

---

## 📄 License

Dual-licensed under:
- Apache License 2.0
- MIT License

---

> **flexicon** — because great systems speak human.