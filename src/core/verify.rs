use crate::handler::ArctgzError;
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub fn verify(
    archive_path: &Path,
    public_key: Option<&[u8]>,
    password: Option<&str>,
) -> Result<(), ArctgzError> {
    let working_path;
    let actual_path = if crate::core::encrypt::is_encrypted(archive_path)? {
        let pw = password.ok_or_else(|| {
            ArctgzError::EncryptionError("Password required for encrypted archive".into())
        })?;
        let tmp = archive_path.with_extension("dec");
        crate::core::encrypt::decrypt_file(archive_path, &tmp, pw)?;
        working_path = tmp;
        &working_path
    } else {
        archive_path
    };

    let result = verify_inner(actual_path, public_key);
    if actual_path != archive_path {
        let _ = fs::remove_file(actual_path);
    }
    result
}

fn verify_inner(archive_path: &Path, public_key: Option<&[u8]>) -> Result<(), ArctgzError> {
    let (manifest, compression) = crate::core::archive::read_manifest(archive_path)?;

    if let Some(pk) = public_key {
        crate::core::sign::verify_manifest(&manifest, pk)?;
    }

    let file = File::open(archive_path)?;
    let decoder = crate::core::archive::make_reader_from_file(file, &compression)?;
    let mut archive = tar::Archive::new(decoder);

    let mut files_found: HashSet<String> = HashSet::new();
    let mut buf = [0u8; 8192];

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if path == "manifest.json" {
            let mut sink = [0u8; 8192];
            while entry.read(&mut sink)? > 0 {}
            continue;
        }

        if !crate::core::archive::is_safe_archive_path(&path) {
            return Err(ArctgzError::VerifyError(format!("Unsafe path: {}", path)));
        }

        let expected = manifest
            .files
            .get(&path)
            .ok_or_else(|| ArctgzError::VerifyError(format!("File '{}' not in manifest", path)))?;

        if expected.is_dir {
            if entry.header().entry_type() != tar::EntryType::Directory {
                return Err(ArctgzError::VerifyError(format!(
                    "Expected directory but got file: {}",
                    path
                )));
            }
            files_found.insert(path);
            continue;
        }

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
