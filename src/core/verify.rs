use crate::handler::{ArctgzError, ArctgzManifest};
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

pub fn verify(archive_path: &Path) -> Result<(), ArctgzError> {
    let file = File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(gz);
    let mut manifest_bytes: Option<Vec<u8>> = None;

    for entry in tar.entries()? {
        let mut entry = entry?;
        if entry.path()?.to_string_lossy() == "manifest.json" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            manifest_bytes = Some(buf);
            break;
        }
    }

    let manifest_json = manifest_bytes.ok_or(ArctgzError::ManifestNotFound)?;
    let manifest: ArctgzManifest = serde_json::from_slice(&manifest_json)?;

    let file2 = File::open(archive_path)?;
    let gz2 = flate2::read::GzDecoder::new(file2);
    let mut tar2 = tar::Archive::new(gz2);
    let mut files_found: HashSet<String> = HashSet::new();

    for entry in tar2.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if path == "manifest.json" {
            continue;
        }

        let p = Path::new(&path);
        if p.is_absolute() || p.components().any(|c| c == Component::ParentDir) {
            return Err(ArctgzError::VerifyError(format!(
                "Unsafe path in archive: {}",
                path
            )));
        }

        let expected = manifest.files.get(&path).ok_or_else(|| {
            ArctgzError::VerifyError(format!(
                "File '{}' in archive is not listed in manifest",
                path
            ))
        })?;

        let mut hasher = Sha512::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = entry.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
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
