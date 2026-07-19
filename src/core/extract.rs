use crate::handler::ArctgzError;
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

pub fn extract(
    archive_path: &Path,
    output_dir: &Path,
    force: bool,
    public_key: Option<&[u8]>,
) -> Result<(), ArctgzError> {
    let (manifest, compression) = crate::core::archive::read_manifest(archive_path)?;

    if let Some(pk) = public_key {
        crate::core::sign::verify_manifest(&manifest, pk)?;
    }

    let file = File::open(archive_path)?;
    let decoder = crate::core::archive::make_reader_from_file(file, &compression)?;
    let mut archive = tar::Archive::new(decoder);

    let mut extracted_files: HashSet<String> = HashSet::new();
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
            return Err(ArctgzError::ExtractError(format!("Unsafe path: {}", path)));
        }

        let expected = manifest.files.get(&path).ok_or_else(|| {
            ArctgzError::ExtractError(format!("File '{}' in archive not listed in manifest", path))
        })?;

        let dest = output_dir.join(&path);

        if expected.is_dir {
            fs::create_dir_all(&dest)?;
            extracted_files.insert(path);
            continue;
        }

        if dest.exists() && !force {
            return Err(ArctgzError::Io(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("File already exists: {}", dest.display()),
            )));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out_file = fs::File::create(&dest)?;
        let mut hasher = Sha512::new();
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out_file.write_all(&buf[..n])?;
            hasher.update(&buf[..n]);
        }
        let actual_hash = hex::encode(hasher.finalize());

        if actual_hash != expected.sha512 {
            let _ = fs::remove_file(&dest);
            return Err(ArctgzError::ChecksumMismatch(
                path,
                expected.sha512.clone(),
                actual_hash,
            ));
        }

        extracted_files.insert(path);
    }

    for expected_path in manifest.files.keys() {
        if !extracted_files.contains(expected_path.as_str()) {
            return Err(ArctgzError::ExtractError(format!(
                "Manifest lists '{}' but not found in archive",
                expected_path
            )));
        }
    }

    Ok(())
}
