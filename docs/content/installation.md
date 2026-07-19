---
title: "Installation"
desc: "How to add arctgz to your Rust project."
---

# Installation

Currently, **arctgz** is not published on [crates.io](https://crates.io). You can still use it as a dependency by referencing the Git repository directly.

---

## 1. Add the Dependency

Open your `Cargo.toml` file and add:

```toml
[dependencies]
arctgz = { git = "https://github.com/mroczect/arctgz.git", tag = "v0.8.1" }
```

Replace `"v0.8.1"` with the latest [Git tag](https://github.com/mroczect/arctgz/tags) if a newer version exists.

> **Security tip:** always pin to a specific tag (like `v0.8.1`) rather than a branch. This protects your build from unexpected breaking changes.

---

## 2. Import the Crate

In your Rust source files, import the items you need:

```rust
use arctgz::{init, compile, extract, verify, /* ... */};
```

All public types and functions live directly under `arctgz::`.

---

## 3. Build Your Project

Run `cargo build` and Cargo will fetch and compile **arctgz** along with its dependencies.

```bash
cargo build
```

---

That's it! You're ready to start using the library.  
Head over to the [Usage](usage.html) guide for examples and the [API Reference](api-reference.html) for a complete list of functions.
