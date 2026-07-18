mod common;
use common::with_temp_home;

use arctgz::{ArctgzError, compile, init, load_config, save_config, verify};
use std::fs;

#[test]
fn verify_valid_archive() {
    with_temp_home(|home| {
        let project = home.join("valid_project");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("hello.txt"), b"world").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["hello.txt".into()];
        save_config(&project, &config).unwrap();

        let archive = compile(&project, None, false).unwrap();
        verify(&archive).unwrap();
    });
}

#[test]
fn verify_checksum_mismatch() {
    with_temp_home(|home| {
        let fake_archive = home.join("fake.artgz");
        let file = fs::File::create(&fake_archive).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        header.set_size(6);
        header.set_path("data.bin").unwrap();
        tar.append_data(&mut header, "data.bin", b"secret".as_ref())
            .unwrap();

        let manifest = serde_json::json!({
            "name": "test",
            "version": "0.1.0",
            "created": chrono::Utc::now().to_rfc3339(),
            "files": {
                "data.bin": {
                    "size": 6,
                    "sha512": "b3a8e0e1f9ab1bfe3a36f231f676f78bb30a519d2b21e6c530c0eee8ebb4a5d0"
                }
            }
        });
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut mheader = tar::Header::new_gnu();
        mheader.set_size(manifest_bytes.len() as u64);
        mheader.set_path("manifest.json").unwrap();
        tar.append_data(&mut mheader, "manifest.json", &manifest_bytes[..])
            .unwrap();

        let enc = tar.into_inner().unwrap();
        enc.finish().unwrap();

        let res = verify(&fake_archive);
        assert!(matches!(res, Err(ArctgzError::ChecksumMismatch(_, _, _))));
    });
}

#[test]
fn verify_missing_manifest() {
    with_temp_home(|home| {
        let project = home.join("no_manifest");
        fs::create_dir_all(&project).unwrap();
        let archive = project.join("no_manifest.artgz");
        let file = fs::File::create(&archive).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        let mut header = tar::Header::new_gnu();
        header.set_size(5);
        header.set_path("test.txt").unwrap();
        tar.append_data(&mut header, "test.txt", b"hello".as_ref())
            .unwrap();
        let enc = tar.into_inner().unwrap();
        enc.finish().unwrap();

        let res = verify(&archive);
        assert!(matches!(res, Err(ArctgzError::ManifestNotFound)));
    });
}

#[test]
fn verify_file_not_in_manifest() {
    with_temp_home(|home| {
        let project = home.join("extra_file");
        fs::create_dir_all(&project).unwrap();
        let archive = project.join("extra.artgz");
        let file = fs::File::create(&archive).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_path("extra.txt").unwrap();
        tar.append_data(&mut header, "extra.txt", b"oops".as_ref())
            .unwrap();

        let manifest = serde_json::json!({
            "name": "test",
            "version": "0.1.0",
            "created": chrono::Utc::now().to_rfc3339(),
            "files": {
                "real.txt": { "size": 4, "sha512": "a" }
            }
        });
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut mheader = tar::Header::new_gnu();
        mheader.set_size(manifest_bytes.len() as u64);
        mheader.set_path("manifest.json").unwrap();
        tar.append_data(&mut mheader, "manifest.json", &manifest_bytes[..])
            .unwrap();

        let enc = tar.into_inner().unwrap();
        enc.finish().unwrap();

        let res = verify(&archive);
        assert!(matches!(res, Err(ArctgzError::VerifyError(_))));
    });
}

#[test]
fn verify_manifest_file_not_in_archive() {
    with_temp_home(|home| {
        let project = home.join("missing_file");
        fs::create_dir_all(&project).unwrap();
        let archive = project.join("missing.artgz");
        let file = fs::File::create(&archive).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        let manifest = serde_json::json!({
            "name": "test",
            "version": "0.1.0",
            "created": chrono::Utc::now().to_rfc3339(),
            "files": {
                "ghost.txt": {
                    "size": 0,
                    "sha512": "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
                }
            }
        });
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let mut mheader = tar::Header::new_gnu();
        mheader.set_size(manifest_bytes.len() as u64);
        mheader.set_path("manifest.json").unwrap();
        tar.append_data(&mut mheader, "manifest.json", &manifest_bytes[..])
            .unwrap();

        let enc = tar.into_inner().unwrap();
        enc.finish().unwrap();

        let res = verify(&archive);
        assert!(matches!(res, Err(ArctgzError::VerifyError(_))));
    });
}
