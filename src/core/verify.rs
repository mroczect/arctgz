use crate::handler::ArctgzError;
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use std::io::Read;
use std::path::Path;

pub fn verify(archive_path: &Path) -> Result<(), ArctgzError> {
    let (manifest, reader) = crate::core::archive::open_archive_file(archive_path)?;
    let mut archive = tar::Archive::new(reader);
    let mut files_found: HashSet<String> = HashSet::new();
    let mut buf = [0u8; 8192];

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if !crate::core::archive::is_safe_archive_path(&path) {
            return Err(ArctgzError::VerifyError(format!("Unsafe path: {}", path)));
        }

        let expected = manifest
            .files
            .get(&path)
            .ok_or_else(|| ArctgzError::VerifyError(format!("File '{}' not in manifest", path)))?;

        let mut hasher = Sha512::new();
        let mut size: u64 = 0;
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            size += n as u64;
        }

        if size != expected.size {
            return Err(ArctgzError::ChecksumMismatch(
                path,
                format!("size mismatch (expected {}, got {})", expected.size, size),
                String::new(),
            ));
        }

        let actual_hash = hex::encode(hasher.finalize());
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
