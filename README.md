# arctgz &middot; [![GitHub tag](https://img.shields.io/github/v/tag/mroczect/arctgz?label=version)](https://github.com/mroczect/arctgz/tags) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**arctgz** is a safety‑first Rust library for the **arctgz archive format**.  
It provides a complete, cross‑platform lifecycle for projects: scaffold, configure, compile (with integrity verification and optional encryption), extract, verify, delta patch, and execute post‑extraction recipes.

---

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [API Reference](#api-reference)
  - [Initialisation &amp; Configuration](#initialisation--configuration)
    - [`init`](#init)
    - [`load_config`](#load_config)
    - [`save_config`](#save_config)
  - [Archive Operations](#archive-operations)
    - [`compile`](#compile)
    - [`extract`](#extract)
    - [`verify`](#verify)
  - [Delta Patching](#delta-patching)
    - [`diff`](#diff)
    - [`patch`](#patch)
  - [Recipe Management](#recipe-management)
    - [`load_recipe`](#load_recipe)
    - [`extract_recipe`](#extract_recipe)
    - [`execute_recipe`](#execute_recipe)
- [Type Reference](#type-reference)
  - [`ArctgzConfig`](#arctgzconfig)
  - [`ArctgzManifest`](#arctgzmanifest)
  - [`FileEntry`](#fileentry)
  - [`ArctgzRecipe`](#arctgzrecipe)
  - [`RecipeStep`](#recipestep)
  - [`Compression`](#compression)
  - [`Encryption`](#encryption)
  - [`ArctgzDelta`](#arctgzdelta)
  - [`DeltaOp`](#deltaop)
  - [`ArctgzError`](#arctgzerror)
- [File Formats](#file-formats)
  - [Project Configuration (`arctgz.init`)](#project-configuration-arctgzinit)
  - [Recipe File (`recipe.json`)](#recipe-file-recipejson)
  - [Delta File (`ArctgzDelta` JSON)](#delta-file-arctgzdelta-json)
- [Security](#security)
- [Environment Variables](#environment-variables)
- [Testing](#testing)
- [Contributing](#contributing)
- [License](#license)

---

## Installation

Add the Git dependency to your `Cargo.toml`:

```toml
[dependencies]
arctgz = { git = "https://github.com/mroczect/arctgz.git", tag = "v0.8.0" }
```

The library is not yet published on crates.io.

---

## Quick Start

```rust
use arctgz::{init, load_config, save_config, compile, extract, verify, Compression};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project = Path::new("./my-project");

    // 1. Scaffold a new project
    init(project, false)?;

    // 2. Customise configuration
    let mut cfg = load_config(project)?;
    cfg.include = vec!["*.txt".into()];          // glob patterns supported
    cfg.compression = Compression::Zstd;         // or Gzip (default)
    save_config(project, &cfg)?;

    // 3. Add some source files
    std::fs::create_dir_all(project.join("include"))?;
    std::fs::write(project.join("include").join("hello.txt"), b"Hello, arctgz!")?;

    // 4. Compile an archive
    let archive = compile(project, None, false, None, None)?;
    //      archive is project/archive.artgz by default

    // 5. Verify the archive
    verify(&archive, None, None)?;

    // 6. Extract to a directory
    let out = Path::new("./out");
    extract(&archive, out, true, None, None)?;

    Ok(())
}
```

For encryption, signatures, delta patching, and recipes, see the detailed sections below.

---

## API Reference

All public functions reside in the crate root (`arctgz::`).

### Initialisation & Configuration

#### `init`

```rust
pub fn init(project_path: &Path, force: bool) -> Result<(), ArctgzError>
```

Creates a new arctgz project at `project_path`.

- Creates the directory (and parents) if needed.
- Creates an empty `include/` subdirectory.
- Writes a default `arctgz.init` file atomically.
- **`force`** – if `true`, allows initialisation into a non‑empty directory. An existing `arctgz.init` still causes `AlreadyInitialized`.
- **Security** – the canonicalised path must lie under the user’s home directory. No filesystem changes occur before validation.

---

#### `load_config`

```rust
pub fn load_config(project_path: &Path) -> Result<ArctgzConfig, ArctgzError>
```

Reads and validates the `arctgz.init` file from a project directory.  
Returns `ConfigNotFound` if missing, `ConfigLoadError` for I/O or JSON issues, or `ConfigValidation` if the config fails validation.

---

#### `save_config`

```rust
pub fn save_config(project_path: &Path, config: &ArctgzConfig) -> Result<(), ArctgzError>
```

Atomically writes a validated configuration back to `arctgz.init` (temp file + rename).  
Automatically calls `config.validate()` before writing.

---

### Archive Operations

#### `compile`

```rust
pub fn compile(
    project_path: &Path,
    output_path: Option<&Path>,   // defaults to project_path/archive.artgz
    force: bool,
    private_key: Option<&[u8]>,   // 32-byte Ed25519 secret key
    password: Option<&str>,       // required if encryption is enabled
) -> Result<PathBuf, ArctgzError>
```

Compiles the project into an **.artgz** archive.

**Behaviour**:

- Loads the project configuration.
- Collects files from `include/` using **glob patterns** (e.g. `**/*.rs`). Empty directories matching patterns are preserved.
- Recursively walks the `include/` directory; symbolic links are rejected.
- Hashes every file in **parallel** (SHA‑512) using `rayon`.
- Builds a `manifest.json` entry with size, hash, and directory flags.
- Optionally signs the manifest with an Ed25519 **private key** (the signature is stored as a hex string in the manifest).
- Compresses the tar stream with the configured compression (`Gzip` or `Zstd`).
- If encryption is enabled (`Encryption::Aes256Gcm`), encrypts the whole archive after compression.
- Writes atomically via a temporary file.

**`output_path`** – if `None`, defaults to `<project_path>/archive.artgz`.  
**`force`** – overwrite an existing output file.  
**`private_key`** – 32‑byte Ed25519 secret key; if `Some`, the manifest will be signed.  
**`password`** – must be `Some` if `config.encryption == Aes256Gcm`.

---

#### `extract`

```rust
pub fn extract(
    archive_path: &Path,
    output_dir: &Path,
    force: bool,
    public_key: Option<&[u8]>,   // 32-byte Ed25519 public key
    password: Option<&str>,      // required if the archive is encrypted
) -> Result<(), ArctgzError>
```

Extracts and verifies an archive.

- Automatically detects **compression** (gzip/zstd) and **encryption** (AES‑256‑GCM magic bytes).
- If encrypted, the archive is decrypted on‑the‑fly to a temporary file.
- Verifies the Ed25519 signature if a public key is supplied.
- Validates every file’s SHA‑512 hash and size against the manifest.
- Rejects unsafe paths (absolute, `..`, empty, `.`).
- Cleans up partially written files on hash mismatch.
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

Standalone integrity check. Reads the archive, verifies every file’s hash and size, ensures manifest consistency, checks for unsafe paths, and optionally verifies a signature and decrypts. No files are written.

---

### Delta Patching

#### `diff`

```rust
pub fn diff(base_archive: &Path, target_archive: &Path) -> Result<ArctgzDelta, ArctgzError>
```

Computes a binary delta between two archives. Ignores metadata files (`arctgz.init`, `recipe.json`).  
Returns an `ArctgzDelta` containing the minimal set of `Add`, `Modify`, and `Delete` operations.

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

- Copies unchanged files from the base.
- Takes modified or added files from the target.
- Validates integrity of both source archives before writing.
- Optionally signs the resulting manifest.

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

Runs the steps of a recipe inside `output_dir`. Supported steps: `Copy`, `MkDir`, `Chmod` (Unix only, error on other platforms), `Remove`.

- All paths are validated against traversal (absolute, `..`, empty, `.`).
- `force` controls whether existing files/directories are overwritten.

---

## Type Reference

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

Serialised with `#[serde(deny_unknown_fields)]` – unknown keys cause an error.

### `ArctgzManifest`

```rust
pub struct ArctgzManifest {
    pub name: String,
    pub version: String,
    pub created: DateTime<Utc>,
    pub compression: Compression,
    pub files: BTreeMap<String, FileEntry>,
    pub signature: Option<String>,   // hex-encoded Ed25519 signature
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

All errors implement `Display` with human‑readable messages.

---

## File Formats

### Project Configuration (`arctgz.init`)

```json
{
  "name": "my-project",
  "version": "1.0.0",
  "include": ["src/**/*.rs", "*.toml"],
  "compression": "gzip",
  "encryption": "None"
}
```

- `include` – array of glob patterns relative to the `include/` directory.
- `compression` – `"gzip"` or `"zstd"`.
- `encryption` – `"None"` or `"Aes256Gcm"`.
- All fields are required; unknown fields cause an error.

### Recipe File (`recipe.json`)

```json
{
  "name": "post-extract-setup",
  "version": "1.0.0",
  "steps": [
    { "action": "mkdir", "path": "logs" },
    { "action": "copy", "from": "default.cfg", "to": "config.cfg" },
    { "action": "chmod", "path": "bin/app", "mode": "755" },
    { "action": "remove", "path": "temp.txt" }
  ]
}
```

- Steps are executed in order.
- `chmod` is only available on Unix; on other platforms it returns an error.

### Delta File (`ArctgzDelta` JSON)

```json
{
  "base_name": "my-app",
  "base_version": "1.0.0",
  "target_name": "my-app",
  "target_version": "1.1.0",
  "base_manifest_hash": "abc123...",
  "target_manifest_hash": "def456...",
  "operations": [
    {
      "op": "add",
      "path": "new_file.txt",
      "size": 42,
      "sha512": "...",
      "is_dir": false
    },
    {
      "op": "modify",
      "path": "changed.txt",
      "size": 99,
      "sha512": "...",
      "is_dir": false
    },
    { "op": "delete", "path": "old.txt" }
  ]
}
```

---

## Security

- **Path confinement**: `init` ensures projects are created inside the home directory.
- **No side effects before validation**: directory creation and file writes only happen after all checks pass.
- **Atomic writes**: configuration, archives, and temporary files use `create_new(true)` or temp+rename patterns.
- **Archive integrity**: SHA‑512 hashes are verified for every file (and file size) during `extract` and `verify`.
- **Ed25519 signatures**: optional manifest signing with 32‑byte keys; verification on extract/verify.
- **AES‑256‑GCM encryption**: optional encryption of the entire archive with a password; key derived via Argon2.
- **Path traversal prevention**: `extract`, `verify`, and recipe steps reject absolute paths, `..`, empty strings, and `.`.
- **Symlink rejection**: symbolic links are never included in archives.
- **Strict JSON validation**: unknown fields in configuration and recipe files are rejected.

---

## Environment Variables

- **`HOME`** (Unix) / **`USERPROFILE`** (Windows) – used by `init` for home directory confinement.

---

## Testing

Run all tests (unit + integration) with:

```bash
cargo fmt --all
cargo clippy -- -D warnings
cargo test
```

CI runs formatting, linting, build, and tests on Linux, macOS, and Windows (including 32‑bit Linux).

---

## Contributing

Please read [`CONTRIBUTING`](CONTRIBUTING) for guidelines on code style, commit messages, pull request process, and more.

---

## License

This project is licensed under the [MIT License](LICENSE).