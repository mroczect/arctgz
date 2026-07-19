---
title: "ArcTgz"
desc: "A next‑generation archive format built with Rust. Super robust, cross‑platform, and designed for speed and safety."
author: "mroczect <mroczect@proton.me>"
repo_url: "https://github.com/mroczect/arctgz.git"
license: "MIT"
---

# ArcTgz

**arctgz** is a safety‑first Rust library for the **arctgz archive format**.  
It provides a complete, cross‑platform lifecycle for your projects: scaffold, configure, compile (with integrity verification and optional encryption), extract, verify, delta patch, and run post‑extraction recipes.

---

## Key Features

- **Scaffold** a new project with a single call – atomically written configuration.
- **Glob‑based include patterns** – `*.txt`, `src/**/*.rs`, etc. Empty directories are preserved.
- **Parallel SHA‑512 hashing** using `rayon` for fast archive creation.
- **Two compression backends** – gzip (default) and zstd.
- **Optional Ed25519 signing** of the archive manifest.
- **Optional AES‑256‑GCM encryption** of the entire archive with Argon2 key derivation.
- **Full integrity verification** on extraction (hash + size), with automatic cleanup on mismatch.
- **Binary delta patching** – `diff` and `patch` for efficient updates.
- **Post‑extraction recipes** – copy, mkdir, chmod, remove steps, validated for path safety.
- **Strict path safety** – archives reject absolute paths, `..`, empty strings, and `.`.
- **Atomic writes** everywhere – configuration, archive, and temporary files.
- **Cross‑platform** – tested on Linux, macOS, and Windows.

---

## Quick Start

```rust
use arctgz::{init, load_config, save_config, compile, extract, verify, Compression};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project = Path::new("./my-project");

    init(project, false)?;

    let mut cfg = load_config(project)?;
    cfg.include = vec!["*.txt".into()];
    cfg.compression = Compression::Zstd;
    save_config(project, &cfg)?;

    std::fs::create_dir_all(project.join("include"))?;
    std::fs::write(project.join("include").join("hello.txt"), b"Hello, arctgz!")?;

    let archive = compile(project, None, false, None, None)?;
    verify(&archive, None, None)?;

    let out = Path::new("./out");
    extract(&archive, out, true, None, None)?;

    Ok(())
}
```

---

## Explore the Docs

- **[Installation](installation.html)** – add arctgz to your `Cargo.toml`
- **[Usage](usage.html)** – detailed examples for all operations
- **[Configuration](configuration.html)** – the `arctgz.init` file format
- **[API Reference](api-reference.html)** – every public function and type
- **[License](license.html)** – MIT

---

Built with Rust and a focus on safety and performance. Contributions welcome! See the [repository](https://github.com/mroczect/arctgz) for the source code and issue tracker.
