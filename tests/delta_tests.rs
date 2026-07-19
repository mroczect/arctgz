mod common;
use common::with_temp_home;

use arctgz::{
    Compression, DeltaOp, compile, diff, extract, init, load_config, patch, save_config, verify,
};
use std::fs;

#[test]
fn delta_add_file() {
    with_temp_home(|home| {
        let base = home.join("base");
        init(&base, false).unwrap();
        let mut config = load_config(&base).unwrap();
        config.include = vec!["hello.txt".into()];
        save_config(&base, &config).unwrap();
        fs::write(base.join("include").join("hello.txt"), b"hello").unwrap();
        let base_archive = compile(&base, None, false, None, None).unwrap();

        let target = home.join("target");
        init(&target, false).unwrap();
        let mut config = load_config(&target).unwrap();
        config.include = vec!["hello.txt".into(), "world.txt".into()];
        save_config(&target, &config).unwrap();
        fs::write(target.join("include").join("hello.txt"), b"hello").unwrap();
        fs::write(target.join("include").join("world.txt"), b"world").unwrap();
        let target_archive = compile(&target, None, false, None, None).unwrap();

        let delta = diff(&base_archive, &target_archive).unwrap();
        // Pastikan ada operasi Add untuk world.txt
        let has_add = delta
            .operations
            .iter()
            .any(|op| matches!(op, DeltaOp::Add { path, .. } if path == "world.txt"));
        assert!(has_add, "Expected an Add operation for world.txt");

        let patched = home.join("patched.artgz");
        patch(&base_archive, &target_archive, &delta, &patched, None).unwrap();
        verify(&patched, None, None).unwrap();

        let out = home.join("out");
        extract(&patched, &out, false, None, None).unwrap();
        assert_eq!(fs::read_to_string(out.join("hello.txt")).unwrap(), "hello");
        assert_eq!(fs::read_to_string(out.join("world.txt")).unwrap(), "world");
    });
}

#[test]
fn delta_modify_file() {
    with_temp_home(|home| {
        let base = home.join("base2");
        init(&base, false).unwrap();
        let mut config = load_config(&base).unwrap();
        config.include = vec!["data.txt".into()];
        save_config(&base, &config).unwrap();
        fs::write(base.join("include").join("data.txt"), b"old").unwrap();
        let base_archive = compile(&base, None, false, None, None).unwrap();

        let target = home.join("target2");
        init(&target, false).unwrap();
        let mut config = load_config(&target).unwrap();
        config.include = vec!["data.txt".into()];
        save_config(&target, &config).unwrap();
        fs::write(target.join("include").join("data.txt"), b"new").unwrap();
        let target_archive = compile(&target, None, false, None, None).unwrap();

        let delta = diff(&base_archive, &target_archive).unwrap();
        assert_eq!(delta.operations.len(), 1);
        assert!(matches!(delta.operations[0], DeltaOp::Modify { .. }));

        let patched = home.join("patched2.artgz");
        patch(&base_archive, &target_archive, &delta, &patched, None).unwrap();
        verify(&patched, None, None).unwrap();

        let out = home.join("out2");
        extract(&patched, &out, false, None, None).unwrap();
        assert_eq!(fs::read_to_string(out.join("data.txt")).unwrap(), "new");
    });
}

#[test]
fn delta_delete_file() {
    with_temp_home(|home| {
        let base = home.join("base3");
        init(&base, false).unwrap();
        let mut config = load_config(&base).unwrap();
        config.include = vec!["a.txt".into(), "b.txt".into()];
        save_config(&base, &config).unwrap();
        fs::write(base.join("include").join("a.txt"), b"a").unwrap();
        fs::write(base.join("include").join("b.txt"), b"b").unwrap();
        let base_archive = compile(&base, None, false, None, None).unwrap();

        let target = home.join("target3");
        init(&target, false).unwrap();
        let mut config = load_config(&target).unwrap();
        config.include = vec!["a.txt".into()];
        save_config(&target, &config).unwrap();
        fs::write(target.join("include").join("a.txt"), b"a").unwrap();
        let target_archive = compile(&target, None, false, None, None).unwrap();

        let delta = diff(&base_archive, &target_archive).unwrap();
        // Pastikan ada operasi Delete untuk b.txt
        let has_delete = delta
            .operations
            .iter()
            .any(|op| matches!(op, DeltaOp::Delete { path } if path == "b.txt"));
        assert!(has_delete, "Expected a Delete operation for b.txt");

        let patched = home.join("patched3.artgz");
        patch(&base_archive, &target_archive, &delta, &patched, None).unwrap();
        verify(&patched, None, None).unwrap();

        let out = home.join("out3");
        extract(&patched, &out, false, None, None).unwrap();
        assert!(out.join("a.txt").exists());
        assert!(!out.join("b.txt").exists());
    });
}

#[test]
fn delta_zstd() {
    with_temp_home(|home| {
        let base = home.join("zbase");
        init(&base, false).unwrap();
        let mut c = load_config(&base).unwrap();
        c.compression = Compression::Zstd;
        c.include = vec!["f.txt".into()];
        save_config(&base, &c).unwrap();
        fs::write(base.join("include").join("f.txt"), b"old").unwrap();
        let base_archive = compile(&base, None, false, None, None).unwrap();

        let target = home.join("ztarget");
        init(&target, false).unwrap();
        let mut c = load_config(&target).unwrap();
        c.compression = Compression::Zstd;
        c.include = vec!["f.txt".into()];
        save_config(&target, &c).unwrap();
        fs::write(target.join("include").join("f.txt"), b"new").unwrap();
        let target_archive = compile(&target, None, false, None, None).unwrap();

        let delta = diff(&base_archive, &target_archive).unwrap();
        let patched = home.join("patched_zstd.artgz");
        patch(&base_archive, &target_archive, &delta, &patched, None).unwrap();
        verify(&patched, None, None).unwrap();
        let out = home.join("out");
        extract(&patched, &out, false, None, None).unwrap();
        assert_eq!(fs::read_to_string(out.join("f.txt")).unwrap(), "new");
    });
}
