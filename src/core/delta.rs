use crate::handler::{ArctgzDelta, ArctgzError, ArctgzManifest, Compression, DeltaOp, FileEntry};
use sha2::{Digest, Sha512};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

const IGNORE_PATHS: &[&str] = &["arctgz.init", "recipe.json"];

pub fn diff(base_archive: &Path, target_archive: &Path) -> Result<ArctgzDelta, ArctgzError> {
    let (base_manifest, _) = crate::core::archive::read_manifest(base_archive)?;
    let (target_manifest, _) = crate::core::archive::read_manifest(target_archive)?;

    let base_hash = compute_manifest_hash(&base_manifest)?;
    let target_hash = compute_manifest_hash(&target_manifest)?;

    let mut ops = Vec::new();
    let mut processed: HashSet<String> = HashSet::new();

    for (path, entry) in &target_manifest.files {
        if IGNORE_PATHS.contains(&path.as_str()) {
            continue;
        }
        match base_manifest.files.get(path) {
            Some(base_entry)
                if base_entry.size == entry.size
                    && base_entry.sha512 == entry.sha512
                    && base_entry.is_dir == entry.is_dir => {}
            Some(_) => {
                ops.push(DeltaOp::Modify {
                    path: path.clone(),
                    size: entry.size,
                    sha512: entry.sha512.clone(),
                    is_dir: entry.is_dir,
                });
            }
            None => {
                ops.push(DeltaOp::Add {
                    path: path.clone(),
                    size: entry.size,
                    sha512: entry.sha512.clone(),
                    is_dir: entry.is_dir,
                });
            }
        }
        processed.insert(path.clone());
    }

    for path in base_manifest.files.keys() {
        if IGNORE_PATHS.contains(&path.as_str()) || processed.contains(path.as_str()) {
            continue;
        }
        if base_manifest.files[path].is_dir {
            let prefix = format!("{}/", path);
            if target_manifest.files.keys().any(|p| p.starts_with(&prefix)) {
                continue;
            }
        }
        ops.push(DeltaOp::Delete { path: path.clone() });
    }

    Ok(ArctgzDelta {
        base_name: base_manifest.name,
        base_version: base_manifest.version,
        target_name: target_manifest.name,
        target_version: target_manifest.version,
        base_manifest_hash: base_hash,
        target_manifest_hash: target_hash,
        operations: ops,
    })
}

pub fn patch(
    base_archive: &Path,
    target_archive: &Path,
    delta: &ArctgzDelta,
    output_path: &Path,
    private_key: Option<&[u8]>,
) -> Result<(), ArctgzError> {
    let (base_manifest, _) = crate::core::archive::read_manifest(base_archive)?;
    let base_hash = compute_manifest_hash(&base_manifest)?;
    if base_hash != delta.base_manifest_hash {
        return Err(ArctgzError::DeltaError(format!(
            "Base archive hash mismatch: expected {}, got {}",
            delta.base_manifest_hash, base_hash
        )));
    }

    let (target_manifest, _) = crate::core::archive::read_manifest(target_archive)?;

    let mut new_files: BTreeMap<String, FileEntry> = BTreeMap::new();
    let mut delete_set: HashSet<String> = HashSet::new();
    let mut modify_set: HashSet<String> = HashSet::new();
    let mut add_set: HashSet<String> = HashSet::new();

    for op in &delta.operations {
        match op {
            DeltaOp::Delete { path } => {
                delete_set.insert(path.clone());
            }
            DeltaOp::Modify {
                path,
                size,
                sha512,
                is_dir,
            } => {
                modify_set.insert(path.clone());
                new_files.insert(
                    path.clone(),
                    FileEntry {
                        size: *size,
                        sha512: sha512.clone(),
                        is_dir: *is_dir,
                    },
                );
            }
            DeltaOp::Add {
                path,
                size,
                sha512,
                is_dir,
            } => {
                add_set.insert(path.clone());
                new_files.insert(
                    path.clone(),
                    FileEntry {
                        size: *size,
                        sha512: sha512.clone(),
                        is_dir: *is_dir,
                    },
                );
            }
        }
    }

    for (path, entry) in &base_manifest.files {
        if IGNORE_PATHS.contains(&path.as_str()) {
            continue;
        }
        if !delete_set.contains(path) && !modify_set.contains(path) {
            new_files.insert(path.clone(), entry.clone());
        }
    }

    for path in IGNORE_PATHS {
        if let Some(entry) = target_manifest.files.get(*path) {
            new_files.insert(path.to_string(), entry.clone());
        }
    }

    let mut new_manifest = ArctgzManifest {
        name: delta.target_name.clone(),
        version: delta.target_version.clone(),
        created: chrono::Utc::now(),
        compression: target_manifest.compression.clone(),
        files: new_files,
        signature: None,
    };

    if let Some(key) = private_key {
        new_manifest.signature = Some(crate::core::sign::sign_manifest(&new_manifest, key)?);
    }

    write_patched_archive(
        output_path,
        &new_manifest,
        &base_manifest,
        base_archive,
        target_archive,
        delta,
    )?;

    Ok(())
}

fn compute_manifest_hash(manifest: &ArctgzManifest) -> Result<String, ArctgzError> {
    let mut clone = manifest.clone();
    clone.signature = None;
    let json = serde_json::to_vec(&clone)?;
    Ok(hex::encode(Sha512::digest(&json)))
}

fn write_patched_archive(
    output_path: &Path,
    new_manifest: &ArctgzManifest,
    base_manifest: &ArctgzManifest,
    base_archive: &Path,
    target_archive: &Path,
    delta: &ArctgzDelta,
) -> Result<(), ArctgzError> {
    let mut delete_set: HashSet<String> = HashSet::new();
    let mut modify_add_set: HashSet<String> = HashSet::new();

    for op in &delta.operations {
        match op {
            DeltaOp::Delete { path } => {
                delete_set.insert(path.clone());
            }
            DeltaOp::Modify { path, .. } => {
                modify_add_set.insert(path.clone());
            }
            DeltaOp::Add { path, .. } => {
                modify_add_set.insert(path.clone());
            }
        }
    }

    let temp_path = output_path.with_extension("tmp");
    let archive_file = File::create(&temp_path)?;

    match new_manifest.compression {
        Compression::Gzip => {
            let encoder =
                flate2::write::GzEncoder::new(archive_file, flate2::Compression::default());
            let mut builder = tar::Builder::new(encoder);
            write_patched_tar(
                &mut builder,
                new_manifest,
                base_manifest,
                base_archive,
                target_archive,
                &delete_set,
                &modify_add_set,
            )?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
        Compression::Zstd => {
            let encoder = zstd::stream::Encoder::new(archive_file, 0)?;
            let mut builder = tar::Builder::new(encoder);
            write_patched_tar(
                &mut builder,
                new_manifest,
                base_manifest,
                base_archive,
                target_archive,
                &delete_set,
                &modify_add_set,
            )?;
            let encoder = builder.into_inner()?;
            encoder.finish()?;
        }
    }

    fs::rename(&temp_path, output_path)?;
    Ok(())
}

fn write_patched_tar<W: Write>(
    builder: &mut tar::Builder<W>,
    new_manifest: &ArctgzManifest,
    base_manifest: &ArctgzManifest,
    base_archive: &Path,
    target_archive: &Path,
    delete_set: &HashSet<String>,
    modify_add_set: &HashSet<String>,
) -> Result<(), ArctgzError> {
    let manifest_json = serde_json::to_string_pretty(new_manifest)?;
    let mut header = tar::Header::new_gnu();
    header
        .set_path("manifest.json")
        .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
    header.set_size(manifest_json.len() as u64);
    builder.append_data(&mut header, "manifest.json", manifest_json.as_bytes())?;

    let file = File::open(base_archive)?;
    let base_compression = &base_manifest.compression;
    let decoder = crate::core::archive::make_reader_from_file(file, base_compression)?;
    let mut base_tar = tar::Archive::new(decoder);

    for entry in base_tar.entries()? {
        let entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if path == "manifest.json" {
            continue;
        }

        if delete_set.contains(&path) || modify_add_set.contains(&path) {
            continue;
        }

        if IGNORE_PATHS.contains(&path.as_str()) {
            continue;
        }

        let expected = base_manifest.files.get(&path).ok_or_else(|| {
            ArctgzError::DeltaError(format!("File '{}' not found in base manifest", path))
        })?;

        if expected.is_dir {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&path)
                .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
            header.set_size(0);
            header.set_entry_type(tar::EntryType::Directory);
            builder.append_data(&mut header, &path, &[][..])?;
            continue;
        }

        let mut data = Vec::with_capacity(expected.size as usize);
        entry.take(expected.size).read_to_end(&mut data)?;
        if data.len() as u64 != expected.size {
            return Err(ArctgzError::DeltaError(format!(
                "File size mismatch for '{}' in base archive: expected {}, got {}",
                path,
                expected.size,
                data.len()
            )));
        }
        let actual_hash = hex::encode(Sha512::digest(&data));
        if actual_hash != expected.sha512 {
            return Err(ArctgzError::DeltaError(format!(
                "Hash mismatch for unchanged file '{}' in base archive",
                path
            )));
        }

        let mut header = tar::Header::new_gnu();
        header
            .set_path(&path)
            .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
        header.set_size(expected.size);
        builder.append_data(&mut header, &path, data.as_slice())?;
    }

    let file2 = File::open(target_archive)?;
    let target_compression = &new_manifest.compression;
    let decoder2 = crate::core::archive::make_reader_from_file(file2, target_compression)?;
    let mut target_tar = tar::Archive::new(decoder2);

    for entry in target_tar.entries()? {
        let entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if path == "manifest.json" {
            continue;
        }

        if !modify_add_set.contains(&path) && !IGNORE_PATHS.contains(&path.as_str()) {
            continue;
        }

        let expected = new_manifest.files.get(&path).ok_or_else(|| {
            ArctgzError::DeltaError(format!("Missing manifest entry for '{}'", path))
        })?;

        if expected.is_dir {
            let mut header = tar::Header::new_gnu();
            header
                .set_path(&path)
                .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
            header.set_size(0);
            header.set_entry_type(tar::EntryType::Directory);
            builder.append_data(&mut header, &path, &[][..])?;
            continue;
        }

        let mut data = Vec::with_capacity(expected.size as usize);
        entry.take(expected.size).read_to_end(&mut data)?;
        if data.len() as u64 != expected.size {
            return Err(ArctgzError::DeltaError(format!(
                "File size mismatch for '{}': expected {}, got {}",
                path,
                expected.size,
                data.len()
            )));
        }

        let mut header = tar::Header::new_gnu();
        header
            .set_path(&path)
            .map_err(|e| ArctgzError::Io(std::io::Error::other(e)))?;
        header.set_size(expected.size);
        builder.append_data(&mut header, &path, data.as_slice())?;
    }

    Ok(())
}
