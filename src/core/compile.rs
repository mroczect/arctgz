use crate::core::config::load_config;
use crate::core::recipe::validate_recipe;
use crate::handler::{
    ArctgzError, ArctgzManifest, ArctgzRecipe, Compression, Encryption, FileEntry,
};
use chrono::Utc;
use rayon::prelude::*;
use sha2::{Digest, Sha512};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn compile(
    project_path: &Path,
    output_path: Option<&Path>,
    force: bool,
    private_key: Option<&[u8]>,
    password: Option<&str>,
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

    let mut entries: Vec<(PathBuf, PathBuf)> = Vec::new();
    let special: HashSet<PathBuf> =
        [PathBuf::from("arctgz.init"), PathBuf::from("recipe.json")].into();

    let init_path = project_path.join("arctgz.init");
    if init_path.exists() {
        entries.push((PathBuf::from("arctgz.init"), init_path));
    }
    let recipe_path = project_path.join("recipe.json");
    if recipe_path.exists() {
        let content = fs::read_to_string(&recipe_path)?;
        let recipe: ArctgzRecipe = serde_json::from_str(&content)?;
        validate_recipe(&recipe)?;
        entries.push((PathBuf::from("recipe.json"), recipe_path));
    }

    for entry in WalkDir::new(&source_dir).into_iter().filter_entry(|_| true) {
        let entry = entry.map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
        let path = entry.path();

        if entry.file_type().is_symlink() {
            return Err(ArctgzError::SymlinkNotAllowed(
                path.to_string_lossy().into_owned(),
            ));
        }

        let rel = path
            .strip_prefix(&source_dir)
            .map_err(|_| ArctgzError::Io(std::io::Error::other("Path strip error")))?;
        if rel.as_os_str().is_empty() {
            continue;
        }

        if special.contains(rel) {
            continue;
        }

        if path.is_dir() {
            let is_empty = WalkDir::new(path).min_depth(1).into_iter().next().is_none();
            if is_empty {
                entries.push((rel.to_path_buf(), path.to_path_buf()));
            }
        } else if path.is_file() {
            let rel_str = rel.to_string_lossy();
            let matched = include_patterns.is_empty()
                || include_patterns.iter().any(|pat| {
                    glob::Pattern::new(pat)
                        .map(|p| p.matches(&rel_str))
                        .unwrap_or(false)
                });
            if matched {
                entries.push((rel.to_path_buf(), path.to_path_buf()));
            }
        }
    }

    let entries_data: Vec<(String, Vec<u8>, String, u64, bool)> = entries
        .par_iter()
        .map(|(rel, abs)| {
            let rel_str = rel.to_string_lossy().into_owned();
            if abs.is_dir() {
                Ok((rel_str, Vec::new(), String::new(), 0u64, true))
            } else {
                let data = fs::read(abs)?;
                let size = data.len() as u64;
                let hash = hex::encode(Sha512::digest(&data));
                Ok((rel_str, data, hash, size, false))
            }
        })
        .collect::<Result<Vec<_>, ArctgzError>>()?;

    let mut manifest_files = BTreeMap::new();
    for (path, _data, hash, size, is_dir) in &entries_data {
        manifest_files.insert(
            path.clone(),
            FileEntry {
                size: *size,
                sha512: hash.clone(),
                is_dir: *is_dir,
            },
        );
    }

    let mut manifest = ArctgzManifest {
        name,
        version,
        created: Utc::now(),
        compression: compression.clone(),
        files: manifest_files,
        signature: None,
    };
    if let Some(key) = private_key {
        manifest.signature = Some(crate::core::sign::sign_manifest(&manifest, key)?);
    }

    let temp_path = output_path.with_extension("tmp");
    let archive_file = File::create(&temp_path)?;

    match compression {
        Compression::Gzip => {
            let encoder =
                flate2::write::GzEncoder::new(archive_file, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);
            write_entries_from_data(&mut builder, &manifest, &entries_data)?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
        Compression::Zstd => {
            let encoder = zstd::stream::Encoder::new(archive_file, 0)?;
            let mut builder = tar::Builder::new(encoder);
            write_entries_from_data(&mut builder, &manifest, &entries_data)?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
    }

    if config.encryption == Encryption::Aes256Gcm {
        let pw = password.ok_or_else(|| {
            ArctgzError::EncryptionError("Password required for encrypted archive".into())
        })?;
        crate::core::encrypt::encrypt_file(&temp_path, &output_path, pw)?;
        fs::remove_file(&temp_path)?;
    } else {
        fs::rename(&temp_path, &output_path)?;
    }
    Ok(output_path)
}

fn write_entries_from_data<W: Write>(
    builder: &mut tar::Builder<W>,
    manifest: &ArctgzManifest,
    entries_data: &[(String, Vec<u8>, String, u64, bool)],
) -> Result<(), ArctgzError> {
    let mjson = serde_json::to_string_pretty(manifest)?;
    let mut header = tar::Header::new_gnu();
    header
        .set_path("manifest.json")
        .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
    header.set_size(mjson.len() as u64);
    builder.append_data(&mut header, "manifest.json", mjson.as_bytes())?;

    for (rel_str, data, _hash, _size, is_dir) in entries_data {
        if *is_dir {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(rel_str)
                .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
            header.set_size(0);
            header.set_entry_type(tar::EntryType::Directory);
            builder.append_data(&mut header, rel_str, &[][..])?;
        } else {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(rel_str)
                .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
            header.set_size(data.len() as u64);
            builder.append_data(&mut header, rel_str, data.as_slice())?;
        }
    }
    Ok(())
}
