use crate::core::config::load_config;
use crate::core::recipe::validate_recipe;
use crate::handler::ArctgzRecipe;
use crate::handler::{ArctgzError, ArctgzManifest, Compression, FileEntry};
use chrono::Utc;
use rayon::prelude::*;
use sha2::{Digest, Sha512};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const BUFFER_SIZE: usize = 64 * 1024;

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

    let metadata: Vec<(String, String, u64, bool)> = entries
        .par_iter()
        .map(|(rel, abs)| {
            let rel_str = rel.to_string_lossy().into_owned();
            if abs.is_dir() {
                Ok::<_, ArctgzError>((rel_str, String::new(), 0u64, true))
            } else {
                let mut file = File::open(abs)?;
                let mut hasher = Sha512::new();
                let mut buf = [0u8; BUFFER_SIZE];
                let mut size = 0u64;
                loop {
                    let n = file.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                    size += n as u64;
                }
                let hash = hex::encode(hasher.finalize());
                Ok((rel_str, hash, size, false))
            }
        })
        .collect::<Result<Vec<_>, ArctgzError>>()?;

    let mut manifest_files = BTreeMap::new();
    for (p, h, s, d) in metadata {
        manifest_files.insert(
            p,
            FileEntry {
                size: s,
                sha512: h,
                is_dir: d,
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
            write_entries(&mut builder, &manifest, &entries)?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
        Compression::Zstd => {
            let encoder = zstd::stream::Encoder::new(archive_file, 0)?;
            let mut builder = tar::Builder::new(encoder);
            write_entries(&mut builder, &manifest, &entries)?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
    }

    fs::rename(&temp_path, &output_path)?;
    Ok(output_path)
}

fn write_entries<W: Write>(
    builder: &mut tar::Builder<W>,
    manifest: &ArctgzManifest,
    entries: &[(PathBuf, PathBuf)],
) -> Result<(), ArctgzError> {
    let mjson = serde_json::to_string_pretty(manifest)?;
    let mut header = tar::Header::new_gnu();
    header
        .set_path("manifest.json")
        .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
    header.set_size(mjson.len() as u64);
    builder.append_data(&mut header, "manifest.json", mjson.as_bytes())?;

    for (rel, abs) in entries {
        let rel_str = rel.to_string_lossy().into_owned();
        if abs.is_dir() {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&rel_str)
                .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
            header.set_size(0);
            header.set_entry_type(tar::EntryType::Directory);
            builder.append_data(&mut header, &rel_str, &[][..])?;
        } else {
            let file = File::open(abs)?;
            let meta = file.metadata()?;
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&rel_str)
                .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
            header.set_size(meta.len());
            let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
            builder.append_data(&mut header, &rel_str, &mut reader)?;
        }
    }
    Ok(())
}
