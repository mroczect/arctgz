use crate::handler::{ArctgzError, ArctgzManifest, Compression};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Component, Path};

pub fn is_safe_archive_path(path: &str) -> bool {
    if path.is_empty() || path == "." {
        return false;
    }
    let p = Path::new(path);
    !p.is_absolute() && !p.components().any(|c| c == Component::ParentDir)
}

pub fn detect_compression(raw: &[u8]) -> Result<Compression, ArctgzError> {
    let magic = raw.get(..4).ok_or_else(|| {
        ArctgzError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "Archive too short to detect compression",
        ))
    })?;

    if magic.starts_with(&[0x1F, 0x8B, 0x08]) {
        return Ok(Compression::Gzip);
    }
    if magic == [0x28, 0xB5, 0x2F, 0xFD] {
        return Ok(Compression::Zstd);
    }

    Err(ArctgzError::Io(std::io::Error::other(
        "Unknown or unsupported compression format",
    )))
}

pub fn make_reader_from_file(
    file: File,
    compression: &Compression,
) -> Result<Box<dyn Read>, ArctgzError> {
    match compression {
        Compression::Gzip => Ok(Box::new(flate2::read::GzDecoder::new(BufReader::new(file)))),
        Compression::Zstd => {
            let decoder = zstd::stream::Decoder::new(BufReader::new(file))?;
            Ok(Box::new(decoder))
        }
    }
}

pub fn read_manifest(archive_path: &Path) -> Result<(ArctgzManifest, Compression), ArctgzError> {
    let mut file = File::open(archive_path)?;

    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;
    let compression = detect_compression(&magic)?;
    file.seek(SeekFrom::Start(0))?;

    let decoder = make_reader_from_file(file, &compression)?;
    let mut archive = tar::Archive::new(decoder);
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path()?.to_string_lossy() == "manifest.json" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            let manifest: ArctgzManifest = serde_json::from_slice(&buf)?;
            return Ok((manifest, compression));
        }
    }
    Err(ArctgzError::ManifestNotFound)
}
