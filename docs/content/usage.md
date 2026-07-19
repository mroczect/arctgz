---
title: "Usage"
desc: "Step-by-step examples for working with arctgz archives."
---

# Using arctgz

This guide walks you through the most common tasks: creating a project, building an archive, verifying its integrity, and applying advanced features like encryption, signing, and delta patching.

All examples assume you have added `arctgz` to your `Cargo.toml` and imported the necessary items:

```rust
use arctgz::*;
use std::path::Path;
```

---

## 1. Starting a New Project

Use `init` to scaffold a new project directory. It creates the folder, an empty `include/` resource directory, and a default `arctgz.init` configuration file.

```rust
init(Path::new("./my-project"), false)?;
```

The second argument (`force`) allows initialisation into a non‑empty directory, but an existing `arctgz.init` file will always cause an `AlreadyInitialized` error.

---

## 2. Configuring the Project

After initialisation you can customise the project. Read the current configuration, modify it, and write it back atomically.

```rust
let mut cfg = load_config(Path::new("./my-project"))?;
cfg.name = "hello-world".into();
cfg.version = "1.0.0".into();
cfg.include = vec!["*.txt".into(), "images/*.png".into()];  // glob patterns
cfg.compression = Compression::Zstd;                         // or Gzip
save_config(Path::new("./my-project"), &cfg)?;
```

See the [Configuration](configuration.html) page for a full description of every field.

---

## 3. Adding Resource Files

Place your resource files inside the `include/` directory. The `include` list in the configuration determines which files (and glob patterns) are collected when building the archive.

```rust
std::fs::create_dir_all("./my-project/include/images")?;
std::fs::write("./my-project/include/readme.txt", b"Hello, arctgz!")?;
std::fs::write("./my-project/include/images/logo.png", &png_bytes)?;
```

Empty directories that match a pattern are preserved as directory entries in the archive.

---

## 4. Building an Archive

Call `compile` to package the project into a single `.artgz` file. By default, the archive is written to `<project>/archive.artgz`.

```rust
let archive_path = compile(
    Path::new("./my-project"),   // project path
    None,                        // output file (None = project/archive.artgz)
    false,                       // force overwrite
    None,                        // Ed25519 private key (optional)
    None,                        // password for encryption (optional)
)?;
println!("Archive created at: {}", archive_path.display());
```

- To use a custom output path, pass `Some(&custom_path)`.
- Set the third argument to `true` to overwrite an existing archive.
- The last two arguments are for signing and encryption – explained later.

---

## 5. Verifying an Archive

Use `verify` to check the integrity of an archive without extracting its contents. It validates every file's SHA‑512 hash, checks file sizes, and ensures the manifest is consistent.

```rust
verify(&archive_path, None, None)?;
```

The extra arguments are for optional Ed25519 signature verification and decryption password – see below.

---

## 6. Extracting an Archive

`extract` unpacks the archive into a target directory while performing the same integrity checks as `verify`. If a file fails validation, it is automatically removed.

```rust
let output_dir = Path::new("./extracted");
extract(&archive_path, output_dir, true, None, None)?;
```

- Set the third argument to `true` to overwrite existing files.
- The function automatically detects compression (gzip/zstd) and encryption (AES‑256‑GCM) and handles them transparently.

---

## 7. Signing and Verifying Archives (Ed25519)

You can add an Ed25519 signature to the manifest to guarantee authenticity.

**Signing during compilation:**

```rust
let private_key: [u8; 32] = // your Ed25519 secret key
let archive = compile(project_path, None, false, Some(&private_key), None)?;
```

The signature is stored as a hex string in the manifest's `signature` field.

**Verifying on extraction (or with `verify`):**

```rust
let public_key: [u8; 32] = // corresponding Ed25519 public key
extract(&archive_path, output_dir, false, Some(&public_key), None)?;
```

If the signature is missing or invalid, the operation fails with `SignatureError`.

---

## 8. Encrypting an Archive (AES‑256‑GCM)

You can encrypt the entire archive with a password. First enable encryption in the configuration:

```rust
let mut cfg = load_config(project_path)?;
cfg.encryption = Encryption::Aes256Gcm;
save_config(project_path, &cfg)?;
```

Then supply a password when compiling, verifying, or extracting:

```rust
let archive = compile(project_path, None, false, None, Some("strong-password"))?;
verify(&archive, None, Some("strong-password"))?;
extract(&archive, &output_dir, false, None, Some("strong-password"))?;
```

Encrypted archives are recognised by the `ARCT` magic bytes and are automatically detected by `extract` and `verify`.

---

## 9. Creating and Applying Binary Deltas

When only a few files change between versions, you can create a delta instead of shipping a full archive.

**Create a delta:**

```rust
let delta = diff(
    &Path::new("v1.0.artgz"),
    &Path::new("v1.1.artgz"),
)?;
```

The resulting `ArctgzDelta` contains only the `Add`, `Modify`, and `Delete` operations needed to transform the base into the target.

**Apply a delta:**

```rust
patch(
    &Path::new("v1.0.artgz"),
    &Path::new("v1.1.artgz"),
    &delta,
    &Path::new("update.artgz"),
    None,   // optional private key for signing
)?;
```

The patched archive is byte‑for‑byte identical to the original target archive.

---

## 10. Using Post‑Extraction Recipes

A recipe file (`recipe.json`) can be placed in the project root. It is automatically included in the archive and can be executed after extraction to set up directories, copy files, change permissions, or remove temporary items.

**Example recipe.json:**

```json
{
  "name": "post-extract-setup",
  "version": "1.0.0",
  "steps": [
    { "action": "mkdir", "path": "logs" },
    { "action": "copy", "from": "config.default", "to": "config.ini" },
    { "action": "chmod", "path": "bin/start.sh", "mode": "755" },
    { "action": "remove", "path": "temp" }
  ]
}
```

Extract the recipe and run it:

```rust
let recipe = extract_recipe(&archive_path)?;
execute_recipe(&output_dir, &recipe, false)?;
```

- The `chmod` step only works on Unix systems; elsewhere it returns an error.
- All paths are validated against traversal attacks.

---

Now you know the main workflows. For a complete list of available functions, types, and error variants, see the [API Reference](api-reference.html). If you need to fine‑tune your project settings, check out the [Configuration](configuration.html) page.
