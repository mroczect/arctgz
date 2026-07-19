mod common;
use common::with_temp_home;

use arctgz::{ArctgzError, compile, extract, init, load_config, save_config};
use std::fs;

#[test]
fn extract_restores_files() {
    with_temp_home(|home| {
        let project = home.join("extract_proj");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("data.txt"), b"sample").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["data.txt".into()];
        save_config(&project, &config).unwrap();

        let archive_path = compile(&project, None, false, None).unwrap();

        let out_dir = home.join("out_extract");
        extract(&archive_path, &out_dir, false, None).unwrap();

        assert!(out_dir.join("arctgz.init").exists());
        assert!(out_dir.join("data.txt").exists());
        let content = fs::read_to_string(out_dir.join("data.txt")).unwrap();
        assert_eq!(content, "sample");
    });
}

#[test]
fn extract_detects_checksum_mismatch() {
    with_temp_home(|home| {
        let project = home.join("mismatch_test");
        fs::create_dir_all(&project).unwrap();
        let archive_path = project.join("bad_checksum.artgz");

        let file = fs::File::create(&archive_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);

        let manifest = serde_json::json!({
            "name": "test",
            "version": "0.1.0",
            "created": chrono::Utc::now().to_rfc3339(),
            "files": {
                "real.txt": {
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

        let mut header = tar::Header::new_gnu();
        header.set_size(6);
        header.set_path("real.txt").unwrap();
        tar.append_data(&mut header, "real.txt", b"secret".as_ref())
            .unwrap();

        let enc = tar.into_inner().unwrap();
        enc.finish().unwrap();

        let out_dir = home.join("out");
        let res = extract(&archive_path, &out_dir, false, None);
        assert!(matches!(res, Err(ArctgzError::ChecksumMismatch(_, _, _))));
    });
}

#[test]
fn extract_no_manifest_errors() {
    with_temp_home(|home| {
        let project = home.join("no_manifest");
        fs::create_dir_all(&project).unwrap();
        let archive_path = project.join("fake.artgz");
        let file = fs::File::create(&archive_path).unwrap();
        let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar = tar::Builder::new(enc);
        let mut header = tar::Header::new_gnu();
        header.set_size(5);
        header.set_path("test.txt").unwrap();
        tar.append_data(&mut header, "test.txt", b"hello".as_ref())
            .unwrap();
        let enc = tar.into_inner().unwrap();
        enc.finish().unwrap();

        let out_dir = home.join("out");
        let res = extract(&archive_path, &out_dir, false, None);
        assert!(matches!(res, Err(ArctgzError::ManifestNotFound)));
    });
}

#[test]
fn extract_overwrite_with_force() {
    with_temp_home(|home| {
        let project = home.join("force_extract");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("file.txt"), b"content").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["file.txt".into()];
        save_config(&project, &config).unwrap();
        let archive_path = compile(&project, None, false, None).unwrap();

        let out_dir = home.join("out");
        fs::create_dir(&out_dir).unwrap();
        fs::write(out_dir.join("file.txt"), b"old").unwrap();
        let res = extract(&archive_path, &out_dir, false, None);
        assert!(res.is_err());
        extract(&archive_path, &out_dir, true, None).unwrap();
        let content = fs::read_to_string(out_dir.join("file.txt")).unwrap();
        assert_eq!(content, "content");
    });
}

#[test]
fn extract_to_nonexistent_dir_creates_it() {
    with_temp_home(|home| {
        let project = home.join("newdir_extract");
        init(&project, false).unwrap();
        let archive_path = compile(&project, None, false, None).unwrap();
        let out_dir = home.join("nonexistent_output");
        extract(&archive_path, &out_dir, false, None).unwrap();
        assert!(out_dir.join("arctgz.init").exists());
    });
}
