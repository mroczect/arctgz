use crate::handler::{ArctgzError, ArctgzManifest, Compression, FileEntry};
use chrono::Utc;
use rayon::prelude::*;
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn compile(
    project_path: &Path,
    output_dir: Option<&Path>,
    force: bool,
) -> Result<PathBuf, ArctgzError> {
    let config = crate::core::config::load_config(project_path)?;
    let include_dir = project_path.join("include");
    if !include_dir.is_dir() {
        return Err(ArctgzError::Io(std::io::Error::other(
            "include directory not found",
        )));
    }

    let mut entries: Vec<(String, PathBuf)> = Vec::new();
    entries.push(("arctgz.init".to_string(), project_path.join("arctgz.init")));
    let recipe_path = project_path.join("recipe.json");
    if recipe_path.exists() {
        crate::core::recipe::load_recipe(project_path)?;
        entries.push(("recipe.json".to_string(), recipe_path));
    }

    for rel_path in &config.include {
        let full_path = include_dir.join(rel_path);
        if !full_path.exists() {
            return Err(ArctgzError::IncludeFileNotFound(format!(
                "File listed in include not found: {}",
                full_path.display()
            )));
        }
        let metadata = fs::symlink_metadata(&full_path)?;
        if metadata.file_type().is_symlink() {
            return Err(ArctgzError::SymlinkNotAllowed(format!(
                "Symlink not allowed: {}",
                full_path.display()
            )));
        }
        if full_path.is_dir() {
            collect_files(&full_path, rel_path, &mut entries)?;
        } else {
            entries.push((rel_path.clone(), full_path));
        }
    }

    let entries_with_data: Vec<(String, Vec<u8>, String, u64)> = entries
        .par_iter()
        .map(|(archive_path, fs_path)| {
            let data = fs::read(fs_path)?;
            let hash = hex::encode(Sha512::digest(&data));
            let size = data.len() as u64;
            Ok::<_, ArctgzError>((archive_path.clone(), data, hash, size))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut manifest = ArctgzManifest {
        name: config.name.clone(),
        version: config.version.clone(),
        created: Utc::now(),
        compression: config.compression.clone(),
        files: BTreeMap::new(),
    };
    for (archive_path, _data, hash, size) in &entries_with_data {
        manifest.files.insert(
            archive_path.clone(),
            FileEntry {
                size: *size,
                sha512: hash.clone(),
            },
        );
    }

    let dist_dir = match output_dir {
        Some(d) => d.to_path_buf(),
        None => project_path.join("dist"),
    };
    fs::create_dir_all(&dist_dir)?;
    let archive_name = format!("{}-{}.artgz", config.name, config.version);
    let archive_path = dist_dir.join(&archive_name);

    if archive_path.exists() && !force {
        return Err(ArctgzError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("archive already exists: {}", archive_path.display()),
        )));
    }

    let temp_path = archive_path.with_extension("tmp");
    let archive_file = File::create(&temp_path)?;

    match config.compression {
        Compression::Gzip => {
            let encoder =
                flate2::write::GzEncoder::new(archive_file, flate2::Compression::default());
            let mut tar_builder = tar::Builder::new(encoder);
            write_tar_entries(&mut tar_builder, &entries_with_data, &manifest)?;
            let encoder = tar_builder.into_inner()?;
            encoder.finish()?;
        }
        Compression::Zstd => {
            let encoder = zstd::stream::Encoder::new(archive_file, 0)?;
            let mut tar_builder = tar::Builder::new(encoder);
            write_tar_entries(&mut tar_builder, &entries_with_data, &manifest)?;
            let encoder = tar_builder.into_inner()?;
            encoder.finish()?;
        }
    }

    fs::rename(&temp_path, &archive_path)?;
    Ok(archive_path)
}

fn write_tar_entries<W: Write>(
    builder: &mut tar::Builder<W>,
    entries: &[(String, Vec<u8>, String, u64)],
    manifest: &ArctgzManifest,
) -> Result<(), ArctgzError> {
    let manifest_json = serde_json::to_string_pretty(manifest)?;
    let mut header = tar::Header::new_gnu();
    header
        .set_path("manifest.json")
        .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
    header.set_size(manifest_json.len() as u64);
    builder.append_data(&mut header, "manifest.json", manifest_json.as_bytes())?;

    for (archive_path, data, _hash, size) in entries {
        let mut header = tar::Header::new_gnu();
        header
            .set_path(archive_path)
            .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
        header.set_size(*size);
        builder.append_data(&mut header, archive_path, data.as_slice())?;
    }
    Ok(())
}

fn collect_files(
    base_dir: &Path,
    prefix: &str,
    entries: &mut Vec<(String, PathBuf)>,
) -> Result<(), ArctgzError> {
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
            ArctgzError::Io(std::io::Error::other(format!(
                "Non-UTF8 filename: {}",
                path.display()
            )))
        })?;
        let archive_path = format!("{}/{}", prefix, file_name);

        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Err(ArctgzError::SymlinkNotAllowed(format!(
                "Symlink not allowed: {}",
                path.display()
            )));
        }

        if path.is_dir() {
            collect_files(&path, &archive_path, entries)?;
        } else {
            entries.push((archive_path, path));
        }
    }
    Ok(())
}
