---
title: "Configuration"
desc: "The arctgz.init project configuration file — format, fields, and validation."
---

# Project Configuration

Every arctgz project contains an `arctgz.init` file that stores its metadata and build instructions.  
This file is JSON and is created automatically by `init()`. You can modify it directly or use the `load_config()` / `save_config()` API.

---

## File Location

```
my-project/
├── arctgz.init        ← configuration file
├── include/           ← resource directory
└── recipe.json        (optional)
```

---

## JSON Structure

```json
{
  "name": "my-project",
  "version": "1.0.0",
  "include": ["src/**/*.rs", "*.toml", "assets/logo.png"],
  "compression": "gzip",
  "encryption": "None"
}
```

All fields are **required**. Unknown keys cause a deserialisation error (thanks to `#[serde(deny_unknown_fields)]`).

---

## Field Reference

### `name`

- **Type:** string
- **Validation:** alphanumeric characters plus `-` and `_`; max 255 characters.
- **Default (when created by `init`):** `"untitled"`

The human‑readable name of your project. It is also used as part of the default archive file name.

### `version`

- **Type:** string
- **Validation:** strict [Semantic Versioning](https://semver.org) (e.g. `1.2.3`, `0.1.0-beta.1`).
- **Default:** `"0.1.0"`

The current version of the project. Changing this does **not** automatically affect compiled archives – you must rebuild after updating.

### `include`

- **Type:** array of strings
- **Validation:** each entry must be a relative path or **glob pattern**, no `..` or absolute paths. Maximum 100 entries.
- **Default:** `[]`

A list of files and directories to include in the archive. All paths are relative to the `include/` directory.

**Glob patterns** are fully supported:

```json
{
  "include": ["*.txt", "docs/**/*.md", "images/icon.png"]
}
```

- `*.txt` – all `.txt` files directly inside `include/`
- `docs/**/*.md` – every `.md` file recursively under `include/docs/`
- `images/icon.png` – a single file

If a pattern matches an empty directory, that directory is preserved as an entry in the archive (e.g. `logs/`).

**Important:** patterns that do **not** match any file produce an `IncludeFileNotFound` error during compilation.

### `compression`

- **Type:** string
- **Allowed values:** `"gzip"`, `"zstd"`
- **Default:** `"gzip"`

The compression algorithm applied to the tar stream inside the archive.

| Value  | Description                    |
| ------ | ------------------------------ |
| `gzip` | Standard gzip (deflate)        |
| `zstd` | Zstandard – better speed/ratio |

You can change it at any time. Already compiled archives record their compression in the manifest, so `extract` and `verify` always use the correct decompressor.

### `encryption`

- **Type:** string
- **Allowed values:** `"None"`, `"Aes256Gcm"`
- **Default:** `"None"`

Whether the final archive should be encrypted with a password.

| Value       | Description                            |
| ----------- | -------------------------------------- |
| `None`      | No encryption (default)                |
| `Aes256Gcm` | AES‑256‑GCM with Argon2 key derivation |

When encryption is enabled, a **password must be supplied** to `compile()`, `verify()`, and `extract()`.  
The password is never stored in the configuration; it is only passed at runtime.

---

## Editing the Configuration

### Programmatically (Rust)

```rust
use arctgz::{load_config, save_config, Compression, Encryption};
use std::path::Path;

let mut cfg = load_config(Path::new("./my-project"))?;
cfg.name = "new-name".into();
cfg.compression = Compression::Zstd;
cfg.encryption = Encryption::Aes256Gcm;
cfg.include = vec!["*.rs".into(), "assets/**".into()];
save_config(Path::new("./my-project"), &cfg)?;
```

### Manually

You can edit the JSON file with any text editor. The next compilation or extraction will automatically pick up the changes.

---

## Example Configuration

```json
{
  "name": "awesome-app",
  "version": "2.1.0",
  "include": ["bin/*", "lib/**/*.so", "config/default.toml"],
  "compression": "zstd",
  "encryption": "Aes256Gcm"
}
```

This configuration will:

- Include everything in `include/bin/` and all `.so` files under `include/lib/`, plus a single config file.
- Compress with Zstandard.
- Encrypt the final archive with AES‑256‑GCM (password must be provided when running `compile`).

---

Next, learn how to use all of this in practice with the [Usage](usage.html) guide, or dive into the full [API Reference](api-reference.html).
