use crate::core::config::load_config;
use crate::core::recipe::validate_recipe;
use crate::handler::ArctgzRecipe;
use crate::handler::{ArctgzError, ArctgzManifest, Compression, FileEntry};
use chrono::Utc;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

struct HashReader<R: Read> {
    inner: R,
    hasher: Sha512,
    size_read: u64,
}

impl<R: Read> Read for HashReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n > 0 {
            self.hasher.update(&buf[..n]);
            self.size_read += n as u64;
        }
        Ok(n)
    }
}

fn build_archive<W: Write>(
    builder: &mut tar::Builder<W>,
    project_path: &Path,
    source_dir: &Path,
    include_patterns: &[String],
    name: &str,
    version: &str,
    private_key: Option<&[u8]>,
) -> Result<(), ArctgzError> {
    let mut manifest_files: BTreeMap<String, FileEntry> = BTreeMap::new();
    let init_path = project_path.join("arctgz.init");
    if init_path.exists() {
        add_file_to_archive(
            builder,
            &mut manifest_files,
            Path::new("arctgz.init"),
            &init_path,
        )?;
    }

    let recipe_path = project_path.join("recipe.json");
    if recipe_path.exists() {
        let content = fs::read_to_string(&recipe_path)?;
        let recipe: ArctgzRecipe = serde_json::from_str(&content)?;
        validate_recipe(&recipe)?;
        add_file_to_archive(
            builder,
            &mut manifest_files,
            Path::new("recipe.json"),
            &recipe_path,
        )?;
    }

    for entry in WalkDir::new(source_dir)
        .into_iter()
        .filter_entry(|e| !is_excluded(e.path(), &[]))
    {
        let entry = entry.map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
        let path = entry.path();

        if entry.file_type().is_symlink() {
            return Err(ArctgzError::SymlinkNotAllowed(
                path.to_string_lossy().into_owned(),
            ));
        }

        if path.is_dir() {
            let rel_path = path
                .strip_prefix(source_dir)
                .map_err(|_| ArctgzError::Io(std::io::Error::other("Path strip error")))?;
            if rel_path.as_os_str().is_empty() {
                continue;
            }

            let is_empty = WalkDir::new(path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_entry(|e| !is_excluded(e.path(), &[]))
                .next()
                .is_none();

            if is_empty {
                add_directory_to_archive(builder, &mut manifest_files, rel_path)?;
            }
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let rel_path = path
            .strip_prefix(source_dir)
            .map_err(|_| ArctgzError::Io(std::io::Error::other("Path strip error")))?;
        let rel_path_str = rel_path.to_string_lossy().into_owned();

        if !include_patterns.is_empty() {
            let matched = include_patterns.iter().any(|pat| {
                glob::Pattern::new(pat)
                    .map(|p| p.matches(&rel_path_str))
                    .unwrap_or(false)
            });
            if !matched {
                continue;
            }
        }

        add_file_to_archive(builder, &mut manifest_files, rel_path, path)?;
    }

    let mut manifest = ArctgzManifest {
        name: name.to_string(),
        version: version.to_string(),
        created: Utc::now(),
        compression: Compression::Gzip,
        files: manifest_files,
        signature: None,
    };

    if let Some(key) = private_key {
        manifest.signature = Some(crate::core::sign::sign_manifest(&manifest, key)?);
    }

    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    let mut header = tar::Header::new_gnu();
    header
        .set_path("manifest.json")
        .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
    header.set_size(manifest_json.len() as u64);
    builder.append_data(&mut header, "manifest.json", manifest_json.as_bytes())?;

    Ok(())
}

fn add_file_to_archive<W: Write>(
    builder: &mut tar::Builder<W>,
    manifest: &mut BTreeMap<String, FileEntry>,
    rel_path: &Path,
    full_path: &Path,
) -> Result<(), ArctgzError> {
    let file = File::open(full_path)?;
    let metadata = file.metadata()?;
    let size = metadata.len();

    let mut hash_reader = HashReader {
        inner: BufReader::new(file),
        hasher: Sha512::new(),
        size_read: 0,
    };

    let path_str = rel_path.to_string_lossy().into_owned();
    let mut header = tar::Header::new_gnu();
    header
        .set_path(&path_str)
        .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
    header.set_size(size);

    builder.append_data(&mut header, &path_str, &mut hash_reader)?;

    if hash_reader.size_read != size {
        return Err(ArctgzError::Io(std::io::Error::other(format!(
            "File size changed during read: expected {} got {}",
            size, hash_reader.size_read
        ))));
    }
    let hash = hex::encode(hash_reader.hasher.finalize());

    manifest.insert(
        path_str,
        FileEntry {
            size,
            sha512: hash,
            is_dir: false,
        },
    );

    Ok(())
}

fn is_excluded(path: &Path, exclude_patterns: &[String]) -> bool {
    if exclude_patterns.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    exclude_patterns.iter().any(|pat| {
        glob::Pattern::new(pat)
            .map(|p| p.matches(&path_str))
            .unwrap_or(false)
    })
}

pub fn compile(
    project_path: &Path,
    output_path: Option<&Path>,
    force: bool,
    private_key: Option<&[u8]>,
) -> Result<PathBuf, ArctgzError> {
    let config = load_config(project_path)?;
    let include_patterns = config.include;
    let name = config.name;
    let version = config.version;
    let compression = config.compression;

    let output_path = match output_path {
        Some(p) => p.to_path_buf(),
        None => project_path.join("archive.artgz"),
    };

    if output_path.exists() && !force {
        return Err(ArctgzError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("Output file already exists: {}", output_path.display()),
        )));
    }

    let source_dir = project_path.join("include");
    if !source_dir.is_dir() {
        return Err(ArctgzError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Include directory not found: {}", source_dir.display()),
        )));
    }

    for pattern in &include_patterns {
        let glob_pattern = source_dir.join(pattern).to_string_lossy().into_owned();
        let matches: Vec<_> = glob::glob(&glob_pattern)
            .map_err(|e| ArctgzError::IncludeFileNotFound(format!("Invalid glob pattern: {}", e)))?
            .filter_map(Result::ok)
            .collect();
        if matches.is_empty() {
            return Err(ArctgzError::IncludeFileNotFound(pattern.clone()));
        }
    }

    let temp_path = output_path.with_extension("tmp");
    let archive_file = File::create(&temp_path)?;

    match compression {
        Compression::Gzip => {
            let encoder =
                flate2::write::GzEncoder::new(archive_file, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);
            build_archive(
                &mut builder,
                project_path,
                &source_dir,
                &include_patterns,
                &name,
                &version,
                private_key,
            )?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
        Compression::Zstd => {
            let encoder = zstd::stream::Encoder::new(archive_file, 0)?;
            let mut builder = tar::Builder::new(encoder);
            build_archive(
                &mut builder,
                project_path,
                &source_dir,
                &include_patterns,
                &name,
                &version,
                private_key,
            )?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
    }

    fs::rename(&temp_path, &output_path)?;
    Ok(output_path)
}

fn add_directory_to_archive<W: Write>(
    builder: &mut tar::Builder<W>,
    manifest: &mut BTreeMap<String, FileEntry>,
    rel_path: &Path,
) -> Result<(), ArctgzError> {
    let path_str = rel_path.to_string_lossy().into_owned();
    let mut header = tar::Header::new_gnu();
    header
        .set_path(&path_str)
        .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
    header.set_size(0);
    header.set_entry_type(tar::EntryType::Directory);
    builder.append_data(&mut header, &path_str, &[][..])?;

    manifest.insert(
        path_str,
        FileEntry {
            size: 0,
            sha512: String::new(),
            is_dir: true,
        },
    );
    Ok(())
}
