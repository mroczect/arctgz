use crate::handler::{ArctgzError, ArctgzManifest, Compression};
use std::io::Read;

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

pub fn make_reader<'a>(
    raw: &'a [u8],
    compression: &Compression,
) -> Result<Box<dyn Read + 'a>, ArctgzError> {
    match compression {
        Compression::Gzip => Ok(Box::new(flate2::read::GzDecoder::new(raw))),
        Compression::Zstd => {
            let decoder = zstd::stream::Decoder::new(raw)?;
            Ok(Box::new(decoder))
        }
    }
}

pub fn open_archive(raw: &[u8]) -> Result<(ArctgzManifest, Box<dyn Read + '_>), ArctgzError> {
    let compression = detect_compression(raw)?;

    let reader1 = make_reader(raw, &compression)?;
    let mut archive1 = tar::Archive::new(reader1);
    let mut manifest_bytes: Option<Vec<u8>> = None;

    for entry in archive1.entries()? {
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

    let reader2 = make_reader(raw, &compression)?;
    Ok((manifest, reader2))
}
