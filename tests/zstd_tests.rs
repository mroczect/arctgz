mod common;
use common::with_temp_home;

use arctgz::{ArctgzError, Compression, compile, extract, init, load_config, save_config, verify};
use std::fs;

#[test]
fn compile_zstd_and_extract() {
    with_temp_home(|home| {
        let project = home.join("zstd_proj");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("hello.txt"), b"zstd world").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["hello.txt".into()];
        config.compression = Compression::Zstd;
        save_config(&project, &config).unwrap();

        let archive = compile(&project, None, false).unwrap();
        assert!(archive.exists());

        verify(&archive).unwrap();

        let out_dir = home.join("out");
        extract(&archive, &out_dir, false).unwrap();
        let content = fs::read_to_string(out_dir.join("hello.txt")).unwrap();
        assert_eq!(content, "zstd world");
    });
}

#[test]
fn verify_zstd_archive() {
    with_temp_home(|home| {
        let project = home.join("zstd_verify");
        init(&project, false).unwrap();
        let mut config = load_config(&project).unwrap();
        config.compression = Compression::Zstd;
        save_config(&project, &config).unwrap();
        let archive = compile(&project, None, false).unwrap();
        verify(&archive).unwrap();
    });
}

#[test]
fn extract_zstd_checksum_mismatch() {
    with_temp_home(|home| {
        let project = home.join("zstd_mismatch");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("file.bin"), b"original").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["file.bin".into()];
        config.compression = Compression::Zstd;
        save_config(&project, &config).unwrap();
        let _archive = compile(&project, None, false).unwrap();

        let fake_archive = home.join("fake_zstd.artgz");
        {
            let file = fs::File::create(&fake_archive).unwrap();
            let encoder = zstd::stream::Encoder::new(file, 0).unwrap();
            let mut tar = tar::Builder::new(encoder);
            let mut header = tar::Header::new_gnu();
            header.set_size(6);
            header.set_path("file.bin").unwrap();
            tar.append_data(&mut header, "file.bin", b"secret".as_ref())
                .unwrap();
            let manifest = serde_json::json!({
                "name": "test",
                "version": "0.1.0",
                "created": chrono::Utc::now().to_rfc3339(),
                "files": {
                    "file.bin": { "size": 6, "sha512": "b3a8e0e1f9ab1bfe3a36f231f676f78bb30a519d2b21e6c530c0eee8ebb4a5d0" }
                }
            });
            let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
            let mut mheader = tar::Header::new_gnu();
            mheader.set_size(manifest_bytes.len() as u64);
            mheader.set_path("manifest.json").unwrap();
            tar.append_data(&mut mheader, "manifest.json", &manifest_bytes[..])
                .unwrap();
            let encoder = tar.into_inner().unwrap();
            encoder.finish().unwrap();
        }

        let out_dir = home.join("out");
        let res = extract(&fake_archive, &out_dir, false);
        assert!(matches!(res, Err(ArctgzError::ChecksumMismatch(_, _, _))));
    });
}
