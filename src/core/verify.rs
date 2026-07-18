use crate::handler::ArctgzError;
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Component, Path};

pub fn verify(archive_path: &Path) -> Result<(), ArctgzError> {
    let raw = std::fs::read(archive_path)?;
    let (manifest, reader) = crate::core::archive::open_archive(&raw)?;

    let mut archive2 = tar::Archive::new(reader);
    let mut files_found: HashSet<String> = HashSet::new();

    for entry in archive2.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if path == "manifest.json" {
            continue;
        }

        let p = Path::new(&path);
        if p.is_absolute()
            || path.is_empty()
            || path == "."
            || p.components().any(|c| c == Component::ParentDir)
        {
            return Err(ArctgzError::VerifyError(format!(
                "Unsafe or empty path in archive: {}",
                path
            )));
        }

        let expected = manifest
            .files
            .get(&path)
            .ok_or_else(|| ArctgzError::VerifyError(format!("File '{}' not in manifest", path)))?;

        let mut content = Vec::new();
        entry.read_to_end(&mut content)?;
        let actual_size = content.len() as u64;
        if actual_size != expected.size {
            return Err(ArctgzError::ChecksumMismatch(
                path,
                format!(
                    "size mismatch (expected {}, got {})",
                    expected.size, actual_size
                ),
                String::new(),
            ));
        }
        let actual_hash = hex::encode(Sha512::digest(&content));
        if actual_hash != expected.sha512 {
            return Err(ArctgzError::ChecksumMismatch(
                path,
                expected.sha512.clone(),
                actual_hash,
            ));
        }

        files_found.insert(path);
    }

    for expected_path in manifest.files.keys() {
        if !files_found.contains(expected_path.as_str()) {
            return Err(ArctgzError::VerifyError(format!(
                "File '{}' listed in manifest but not found in archive",
                expected_path
            )));
        }
    }

    Ok(())
}
