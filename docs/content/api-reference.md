---
title: "API Reference"
desc: "Complete reference of all public functions, types, and error variants in the arctgz library."
---

# API Reference

All public items are exported directly from the crate root.  
Import them with:

```rust
use arctgz::{
    init, compile, extract, verify, diff, patch,
    load_config, save_config, load_recipe, extract_recipe, execute_recipe,
    ArctgzConfig, ArctgzManifest, ArctgzRecipe, ArctgzDelta,
    FileEntry, RecipeStep, Compression, Encryption, DeltaOp,
    ArctgzError,
};
```

---

## Functions

### Project Initialization & Configuration

#### `init`

```rust
pub fn init(project_path: &Path, force: bool) -> Result<(), ArctgzError>
```

Creates a new arctgz project at `project_path`.

- Creates the directory (and parents) if needed.
- Creates an empty `include/` subdirectory.
- Writes a default `arctgz.init` configuration file atomically.
- **Security:** the canonicalized path must lie under the user's home directory.
- **`force`** – if `true`, skips the non‑empty directory check. An existing `arctgz.init` still causes `AlreadyInitialized`.

**Errors:**

- `InvalidPath` – empty path.
- `PathNotAllowed` – path outside home directory.
- `DirectoryNotEmpty` – directory not empty and `force` is `false`.
- `AlreadyInitialized` – `arctgz.init` already exists.
- `Io` – filesystem errors.

---

#### `load_config`

```rust
pub fn load_config(project_path: &Path) -> Result<ArctgzConfig, ArctgzError>
```

Reads and validates the `arctgz.init` file from a project directory.

**Errors:**

- `ConfigNotFound` – file does not exist.
- `ConfigLoadError` – read error or invalid JSON.
- `ConfigValidation` – invalid content.

---

#### `save_config`

```rust
pub fn save_config(project_path: &Path, config: &ArctgzConfig) -> Result<(), ArctgzError>
```

Atomically writes a validated `ArctgzConfig` back to `arctgz.init` (temp file + rename).  
Automatically validates the config before writing.

**Errors:** same as `load_config` plus `ConfigSaveError` on write/rename failure.

---

### Archive Operations

#### `compile`

```rust
pub fn compile(
    project_path: &Path,
    output_path: Option<&Path>,
    force: bool,
    private_key: Option<&[u8]>,
    password: Option<&str>,
) -> Result<PathBuf, ArctgzError>
```

Compiles the project into an `.artgz` archive.

**Behaviour:**

- Loads configuration from `project_path/arctgz.init`.
- Collects files from `project_path/include/` using **glob patterns** defined in `include`.
- Preserves empty directories that match a pattern.
- Rejects symbolic links (returns `SymlinkNotAllowed`).
- Computes SHA‑512 hashes of all files in **parallel** (using `rayon`).
- Builds a `manifest.json` with size, hash, and directory flags.
- Optionally signs the manifest with an Ed25519 private key (32 bytes).
- Compresses the tar stream (gzip or zstd, according to configuration).
- If `config.encryption == Encryption::Aes256Gcm`, encrypts the whole archive with AES‑256‑GCM using the provided password.
- Writes the final file atomically via a temporary file.

**Parameters:**

- `project_path` – root directory of the project.
- `output_path` – if `Some`, the path where the archive will be written; if `None`, defaults to `project_path/archive.artgz`.
- `force` – if `true`, overwrites an existing output file; otherwise returns `Io(AlreadyExists)`.
- `private_key` – 32‑byte Ed25519 signing key for signing the manifest; `None` disables signing.
- `password` – password for encryption (required if `encryption` is set to `Aes256Gcm`; ignored otherwise).

**Returns:** the actual path of the created archive.

**Errors:**

- `ConfigNotFound`, `ConfigLoadError`, `ConfigValidation` – config issues.
- `Io` – filesystem errors.
- `SymlinkNotAllowed` – symbolic link in `include/`.
- `IncludeFileNotFound` – a glob pattern matched no files.
- `KeyError` – invalid private key length.
- `EncryptionError` – password missing or encryption failure.
- `SignatureError` – signing failure (unlikely).
- `DeltaError` – internal encoding error (should not happen).

---

#### `extract`

```rust
pub fn extract(
    archive_path: &Path,
    output_dir: &Path,
    force: bool,
    public_key: Option<&[u8]>,
    password: Option<&str>,
) -> Result<(), ArctgzError>
```

Extracts and verifies an archive.

**Behaviour:**

- Auto‑detects compression (gzip/zstd) and encryption (by magic bytes).
- If encrypted, decrypts to a temporary file using the provided password.
- Verifies the Ed25519 signature if a public key is given.
- For each file in the archive:
  - Checks that the path is safe (no absolute, `..`, empty, or `.`).
  - Compares the actual SHA‑512 hash and size against the manifest.
  - On mismatch, removes any partially written file and returns an error.
- If `force` is `true`, overwrites existing files; otherwise returns `Io(AlreadyExists)`.
- Creates parent directories as needed.
- Handles directory entries correctly.

**Parameters:**

- `archive_path` – path to the `.artgz` file.
- `output_dir` – destination directory.
- `force` – allow overwriting existing files.
- `public_key` – 32‑byte Ed25519 public key for signature verification; `None` skips verification.
- `password` – password for decryption; required if the archive is encrypted, otherwise ignored.

**Errors:**

- `ManifestNotFound` – archive does not contain `manifest.json`.
- `SignatureError` – signature invalid or missing.
- `EncryptionError` – wrong password or decryption failure.
- `ChecksumMismatch` – hash or size mismatch.
- `ExtractError` – unsafe path, directory/file type mismatch, or manifest inconsistency.
- `Io` – filesystem errors.

---

#### `verify`

```rust
pub fn verify(
    archive_path: &Path,
    public_key: Option<&[u8]>,
    password: Option<&str>,
) -> Result<(), ArctgzError>
```

Standalone integrity check. Reads the archive and performs all the same validations as `extract`, but writes no files.

**Parameters:** same as `extract` (without `force` and `output_dir`).

**Errors:** same as `extract` (excluding `Io` related to file creation, but still may have `Io` for reading).

---

### Delta Patching

#### `diff`

```rust
pub fn diff(base_archive: &Path, target_archive: &Path) -> Result<ArctgzDelta, ArctgzError>
```

Computes a binary delta between two archives.

**Behaviour:**

- Reads both manifests.
- Compares file entries by path, size, hash, and directory flag.
- Generates `Add`, `Modify`, or `Delete` operations.
- Ignores metadata files (`recipe.json`; `arctgz.init` is **not** ignored because it can change between versions).

**Parameters:**

- `base_archive` – path to the base archive.
- `target_archive` – path to the newer archive.

**Returns:** an `ArctgzDelta` containing the list of operations.

**Errors:** same as `read_manifest` (I/O, JSON, etc.).

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

Applies a previously computed delta to produce a new archive identical to the target.

**Behaviour:**

- Reads the base archive and the target archive (to copy files from both).
- Copies unchanged files from the base, changed/added files from the target.
- Deletes files as specified.
- Builds a new manifest with the target's metadata.
- Optionally signs the new manifest.
- Writes the result atomically.

**Parameters:**

- `base_archive` – path to the base archive.
- `target_archive` – path to the target archive (used as source for modified/added files).
- `delta` – the delta returned by `diff`.
- `output_path` – where to write the patched archive.
- `private_key` – optional signing key.

**Errors:**

- `DeltaError` – manifest hash mismatch, missing entries, size/hash verification failures.
- `Io`, `KeyError`, `SignatureError` – as in `compile`.

---

### Recipe Management

#### `load_recipe`

```rust
pub fn load_recipe(project_path: &Path) -> Result<Option<ArctgzRecipe>, ArctgzError>
```

Loads and validates `recipe.json` from a project directory. Returns `None` if the file is absent.

**Errors:**

- `Io`, `Json`, `RecipeInvalid`.

---

#### `extract_recipe`

```rust
pub fn extract_recipe(archive_path: &Path) -> Result<ArctgzRecipe, ArctgzError>
```

Extracts and validates the embedded `recipe.json` from an archive. Handles compression and encryption transparently.

**Errors:**

- `RecipeNotFound` – no `recipe.json` in the archive.
- `Io`, `Json`, `RecipeInvalid`.

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

**Supported steps:**

- `Copy { from, to }` – copies `from` to `to` (both relative to `output_dir`). If `from` is a directory, copies recursively.
- `MkDir { path }` – creates a directory (and parents).
- `Chmod { path, mode }` – sets Unix permissions (octal mode). Returns error on non‑Unix platforms.
- `Remove { path }` – deletes a file or directory (must be empty).

All paths are validated against traversal attacks (`..` and absolute paths are rejected).  
`force` controls whether existing files/directories are overwritten or cause an error.

**Errors:**

- `RecipeInvalid` – invalid step or path.
- `RecipeExecutionError` – step failed (e.g., missing source, permission denied).
- `Io` – filesystem errors.

---

## Types

### `ArctgzConfig`

```rust
pub struct ArctgzConfig {
    pub name: String,
    pub version: String,
    pub include: Vec<String>,
    pub compression: Compression,
    pub encryption: Encryption,
}
```

Project configuration. Serialised as JSON with `deny_unknown_fields`.

**Validation:**

- `name` – alphanumeric + `-` and `_`, max 255 chars.
- `version` – strict semver.
- `include` – relative glob patterns, max 100, no `..` or absolute paths.
- `compression` – `Gzip` or `Zstd`.
- `encryption` – `None` or `Aes256Gcm`.

Default values (created by `init`):  
`name` = `"untitled"`, `version` = `"0.1.0"`, `include` = `[]`, `compression` = `Gzip`, `encryption` = `None`.

---

### `ArctgzManifest`

```rust
pub struct ArctgzManifest {
    pub name: String,
    pub version: String,
    pub created: DateTime<Utc>,
    pub compression: Compression,
    pub files: BTreeMap<String, FileEntry>,
    pub signature: Option<String>,
}
```

Embedded in every archive as `manifest.json`.

- `signature` – hex‑encoded Ed25519 signature of the manifest (without the signature field).

---

### `FileEntry`

```rust
pub struct FileEntry {
    pub size: u64,
    pub sha512: String,
    pub is_dir: bool,
}
```

Describes a file or directory entry in the manifest.

---

### `ArctgzRecipe`

```rust
pub struct ArctgzRecipe {
    pub name: String,
    pub version: String,
    pub steps: Vec<RecipeStep>,
}
```

Post‑extraction recipe.

---

### `RecipeStep`

```rust
pub enum RecipeStep {
    Copy   { from: String, to: String },
    MkDir  { path: String },
    Chmod  { path: String, mode: String },
    Remove { path: String },
}
```

Deserialised with `#[serde(tag = "action")]`.

---

### `Compression`

```rust
pub enum Compression {
    Gzip,
    Zstd,
}
```

Default: `Gzip`.

---

### `Encryption`

```rust
pub enum Encryption {
    None,
    Aes256Gcm,
}
```

Default: `None`.

---

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

Represents the difference between two archives.

---

### `DeltaOp`

```rust
pub enum DeltaOp {
    Add    { path: String, size: u64, sha512: String, is_dir: bool },
    Modify { path: String, size: u64, sha512: String, is_dir: bool },
    Delete { path: String },
}
```

Serialised with `#[serde(tag = "op")]`.

---

## Error Types

### `ArctgzError`

All possible errors returned by the library.

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

Each variant provides a human‑readable description via `Display`.  
`Io` and `Json` are automatically converted from the underlying error types using `From`.

---

For practical examples, see the [Usage](usage.html) page. To understand configuration, check [Configuration](configuration.html).
