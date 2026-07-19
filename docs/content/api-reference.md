---
title: "API Reference"
desc: "Complete reference of all public functions, types, and error variants in the arctgz library."
---

# API Reference

All public items live directly under the `arctgz` crate root.  
Import what you need:

```rust
use arctgz::{init, compile, extract, verify, diff, patch, /* ... */};
```

---

## Functions

### Initialisation & Configuration

#### `init`

```rust
pub fn init(project_path: &Path, force: bool) -> Result<(), ArctgzError>
```

Creates a new arctgz project at `project_path`.

- Creates the directory (and parents) if needed.
- Creates an empty `include/` subdirectory.
- Writes a default `arctgz.init` configuration file atomically.
- **`force`** – if `true`, skips the non‑empty directory check. An existing `arctgz.init` still causes `AlreadyInitialized`.
- **Security** – the canonicalised path must lie under the user's home directory. No filesystem changes occur before validation.

---

#### `load_config`

```rust
pub fn load_config(project_path: &Path) -> Result<ArctgzConfig, ArctgzError>
```

Reads and validates the `arctgz.init` file from a project directory.  
Errors: `ConfigNotFound`, `ConfigLoadError`, `ConfigValidation`.

---

#### `save_config`

```rust
pub fn save_config(project_path: &Path, config: &ArctgzConfig) -> Result<(), ArctgzError>
```

Atomically writes a validated `ArctgzConfig` back to `arctgz.init` (temp file + rename).  
Automatically calls `config.validate()` before writing.

---

### Archive Operations

#### `compile`

```rust
pub fn compile(
    project_path: &Path,
    output_path: Option<&Path>,   // defaults to project_path/archive.artgz
    force: bool,
    private_key: Option<&[u8]>,   // 32‑byte Ed25519 secret key
    password: Option<&str>,       // required if encryption is enabled
) -> Result<PathBuf, ArctgzError>
```

Compiles the project into an **`.artgz`** archive.

**Behaviour**:

- Loads configuration.
- Collects files from `include/` using **glob patterns**. Empty directories matching patterns are preserved.
- Recursively walks `include/`; symbolic links are rejected.
- Hashes every file in **parallel** (SHA‑512) with `rayon`.
- Builds a `manifest.json` with size, hash, and directory flags.
- Optionally signs the manifest with an Ed25519 private key.
- Compresses the tar stream (gzip or zstd).
- If `config.encryption == Aes256Gcm`, encrypts the whole archive after compression.
- Writes atomically via a temporary file.

---

#### `extract`

```rust
pub fn extract(
    archive_path: &Path,
    output_dir: &Path,
    force: bool,
    public_key: Option<&[u8]>,   // 32‑byte Ed25519 public key
    password: Option<&str>,      // required if archive is encrypted
) -> Result<(), ArctgzError>
```

Extracts and verifies an archive.

- Auto‑detects compression and encryption.
- Decrypts on the fly if needed (temporary file).
- Verifies Ed25519 signature if public key is given.
- Validates SHA‑512 hashes and sizes against the manifest.
- Rejects unsafe paths (absolute, `..`, empty, `.`).
- Removes partially written files on hash mismatch.
- `force` controls overwriting of existing files.

---

#### `verify`

```rust
pub fn verify(
    archive_path: &Path,
    public_key: Option<&[u8]>,
    password: Option<&str>,
) -> Result<(), ArctgzError>
```

Standalone integrity check. Reads the archive and validates everything `extract` does, but writes no files.  
Useful for pre‑extraction validation or automated checks.

---

### Delta Patching

#### `diff`

```rust
pub fn diff(base_archive: &Path, target_archive: &Path) -> Result<ArctgzDelta, ArctgzError>
```

Computes a binary delta between two archives.  
Ignores metadata files (`arctgz.init`, `recipe.json`).  
Returns an `ArctgzDelta` with `Add`, `Modify`, and `Delete` operations.

---

#### `patch`

```rust
pub fn patch(
    base_archive: &Path,
    target_archive: &Path,
    delta: &ArctgzDelta,
    output_path: &Path,
    private_key: Option<&[u8]>,
) -> Result<(), ArctgzError>
```

Applies a previously computed delta to the base archive, producing a new archive identical to the original target.  
Copies unchanged files from base, takes changed files from target.  
Optionally signs the new manifest.

---

### Recipe Management

#### `load_recipe`

```rust
pub fn load_recipe(project_path: &Path) -> Result<Option<ArctgzRecipe>, ArctgzError>
```

Loads and validates `recipe.json` from a project directory. Returns `None` if the file is absent.

---

#### `extract_recipe`

```rust
pub fn extract_recipe(archive_path: &Path) -> Result<ArctgzRecipe, ArctgzError>
```

Extracts and validates the embedded `recipe.json` from an archive. Handles all compression and encryption transparently.

---

#### `execute_recipe`

```rust
pub fn execute_recipe(
    output_dir: &Path,
    recipe: &ArctgzRecipe,
    force: bool,
) -> Result<(), ArctgzError>
```

Runs the steps of a recipe inside `output_dir`.  
Supported steps: `Copy`, `MkDir`, `Chmod` (Unix only, error on other platforms), `Remove`.  
All paths are validated against traversal; `force` controls overwriting.

---

## Types

### `ArctgzConfig`

```rust
pub struct ArctgzConfig {
    pub name: String,                  // alphanumeric, '-' and '_', max 255
    pub version: String,               // strict semver
    pub include: Vec<String>,          // relative glob patterns, max 100
    pub compression: Compression,      // Gzip (default) or Zstd
    pub encryption: Encryption,        // None (default) or Aes256Gcm
}
```

Serialised with `deny_unknown_fields` – unknown keys cause an error.  
Created by `init()` with defaults; can be modified via `load_config()` / `save_config()`.

### `ArctgzManifest`

```rust
pub struct ArctgzManifest {
    pub name: String,
    pub version: String,
    pub created: DateTime<Utc>,
    pub compression: Compression,
    pub files: BTreeMap<String, FileEntry>,
    pub signature: Option<String>,   // hex‑encoded Ed25519 signature
}
```

Embedded in every archive as `manifest.json`.

### `FileEntry`

```rust
pub struct FileEntry {
    pub size: u64,
    pub sha512: String,
    pub is_dir: bool,
}
```

Represents a single file or directory entry in the manifest.

### `ArctgzRecipe`

```rust
pub struct ArctgzRecipe {
    pub name: String,
    pub version: String,
    pub steps: Vec<RecipeStep>,
}
```

Serialised with `deny_unknown_fields`.

### `RecipeStep`

```rust
pub enum RecipeStep {
    Copy   { from: String, to: String },
    MkDir  { path: String },
    Chmod  { path: String, mode: String },  // octal string, Unix only
    Remove { path: String },
}
```

Tagged with `"action"` in JSON.

### `Compression`

```rust
pub enum Compression {
    Gzip, // default
    Zstd,
}
```

### `Encryption`

```rust
pub enum Encryption {
    None,        // default
    Aes256Gcm,
}
```

### `ArctgzDelta`

```rust
pub struct ArctgzDelta {
    pub base_name: String,
    pub base_version: String,
    pub target_name: String,
    pub target_version: String,
    pub base_manifest_hash: String,
    pub target_manifest_hash: String,
    pub operations: Vec<DeltaOp>,
}
```

### `DeltaOp`

```rust
pub enum DeltaOp {
    Add    { path: String, size: u64, sha512: String, is_dir: bool },
    Modify { path: String, size: u64, sha512: String, is_dir: bool },
    Delete { path: String },
}
```

Tagged with `"op"` in JSON.

---

## Error Types

### `ArctgzError`

```rust
pub enum ArctgzError {
    Io(std::io::Error),
    Json(serde_json::Error),
    AlreadyInitialized,
    InvalidPath(String),
    PathNotAllowed(String),
    DirectoryNotEmpty(String),
    ConfigValidation(String),
    ConfigNotFound(String),
    ConfigLoadError(String),
    ConfigSaveError(String),
    SymlinkNotAllowed(String),
    IncludeFileNotFound(String),
    ManifestNotFound,
    ChecksumMismatch(String, String, String),
    ExtractError(String),
    RecipeNotFound,
    RecipeInvalid(String),
    RecipeExecutionError(String),
    VerifyError(String),
    SignatureError(String),
    KeyError(String),
    DeltaError(String),
    EncryptionError(String),
}
```

All variants implement `Display` with human‑readable messages.  
The `?` operator can be used with `std::io::Error` and `serde_json::Error` thanks to `From` implementations.

---

For practical examples, head to the [Usage](usage.html) page. To understand configuration options, see [Configuration](configuration.html).
