use crate::handler::ArctgzError;
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Component, Path};

pub fn extract(archive_path: &Path, output_dir: &Path, force: bool) -> Result<(), ArctgzError> {
    let raw = std::fs::read(archive_path)?;

    let (manifest, reader) = crate::core::archive::open_archive(&raw)?;

    let mut archive2 = tar::Archive::new(reader);
    let mut file_entries: Vec<(String, Vec<u8>)> = Vec::new();

    for entry in archive2.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if path == "manifest.json" {
            continue;
        }

        let p = Path::new(&path);
        if p.is_absolute() || p.components().any(|c| c == Component::ParentDir) {
            return Err(ArctgzError::ExtractError(format!("Unsafe path: {}", path)));
        }

        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        file_entries.push((path, buf));
    }

    let mut extracted_files: HashSet<String> = HashSet::new();
    for (file_path, content) in &file_entries {
        let expected = manifest.files.get(file_path.as_str()).ok_or_else(|| {
            ArctgzError::ExtractError(format!(
                "File '{}' in archive not listed in manifest",
                file_path
            ))
        })?;

        let actual_hash = hex::encode(Sha512::digest(content.as_slice()));
        if actual_hash != expected.sha512 {
            return Err(ArctgzError::ChecksumMismatch(
                file_path.clone(),
                expected.sha512.clone(),
                actual_hash,
            ));
        }
        extracted_files.insert(file_path.clone());
    }

    for expected_path in manifest.files.keys() {
        if !extracted_files.contains(expected_path.as_str()) {
            return Err(ArctgzError::ExtractError(format!(
                "Manifest lists '{}' but not found in archive",
                expected_path
            )));
        }
    }

    fs::create_dir_all(output_dir)?;
    for (file_path, content) in &file_entries {
        let dest = output_dir.join(file_path);
        if dest.exists() && !force {
            return Err(ArctgzError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("File already exists: {}", dest.display()),
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, content)?;
    }

    Ok(())
}
