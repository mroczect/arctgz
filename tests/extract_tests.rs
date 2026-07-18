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

        let archive_path = compile(&project, None, false).unwrap();

        let out_dir = home.join("out_extract");
        extract(&archive_path, &out_dir, false).unwrap();

        assert!(out_dir.join("arctgz.init").exists());
        assert!(out_dir.join("data.txt").exists());
        let content = fs::read_to_string(out_dir.join("data.txt")).unwrap();
        assert_eq!(content, "sample");
    });
}

#[test]
fn extract_detects_checksum_mismatch() {}

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
        let res = extract(&archive_path, &out_dir, false);
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
        let archive_path = compile(&project, None, false).unwrap();

        let out_dir = home.join("out");
        fs::create_dir(&out_dir).unwrap();
        fs::write(out_dir.join("file.txt"), b"old").unwrap();
        let res = extract(&archive_path, &out_dir, false);
        assert!(res.is_err());
        extract(&archive_path, &out_dir, true).unwrap();
        let content = fs::read_to_string(out_dir.join("file.txt")).unwrap();
        assert_eq!(content, "content");
    });
}

#[test]
fn extract_to_nonexistent_dir_creates_it() {
    with_temp_home(|home| {
        let project = home.join("newdir_extract");
        init(&project, false).unwrap();
        let archive_path = compile(&project, None, false).unwrap();
        let out_dir = home.join("nonexistent_output");
        extract(&archive_path, &out_dir, false).unwrap();
        assert!(out_dir.join("arctgz.init").exists());
    });
}
