mod common;
use common::with_temp_home;

use arctgz::{ArctgzError, compile, extract, init, load_config, save_config};
use std::fs;

#[test]
fn compile_uses_include_list() {
    with_temp_home(|home| {
        let project = home.join("include_list_proj");
        init(&project, false).unwrap();

        fs::create_dir_all(project.join("include").join("assets")).unwrap();
        fs::write(
            project.join("include").join("assets").join("pic.png"),
            b"png",
        )
        .unwrap();
        fs::write(project.join("include").join("hello.txt"), b"hello").unwrap();

        let mut config = load_config(&project).unwrap();
        config.include = vec!["hello.txt".into()];
        save_config(&project, &config).unwrap();

        let archive_path = compile(&project, None, false, None).unwrap();
        assert!(archive_path.exists());

        let f = fs::File::open(&archive_path).unwrap();
        let gz = flate2::read::GzDecoder::new(f);
        let mut archive = tar::Archive::new(gz);
        let entries: Vec<_> = archive
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(entries.contains(&"arctgz.init".to_string()));
        assert!(entries.contains(&"hello.txt".to_string()));
        assert!(!entries.contains(&"assets/pic.png".to_string()));
    });
}

#[test]
fn compile_rejects_symlink() {
    with_temp_home(|home| {
        let project = home.join("symlink_proj");
        init(&project, false).unwrap();
        let include_dir = project.join("include");

        fs::write(include_dir.join("real.txt"), b"real").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(include_dir.join("real.txt"), include_dir.join("link.txt"))
            .unwrap();

        let mut config = load_config(&project).unwrap();
        config.include = vec!["link.txt".into()];
        save_config(&project, &config).unwrap();

        let res = compile(&project, None, false, None);
        #[cfg(unix)]
        assert!(matches!(res, Err(ArctgzError::SymlinkNotAllowed(_))));
        #[cfg(not(unix))]
        assert!(matches!(res, Err(ArctgzError::IncludeFileNotFound(_))));
    });
}

#[test]
fn compile_force_overwrites_existing_archive() {
    with_temp_home(|home| {
        let project = home.join("force_compile");
        init(&project, false).unwrap();
        compile(&project, None, false, None).unwrap();
        let res = compile(&project, None, false, None);
        assert!(res.is_err());
        let path = compile(&project, None, true, None).unwrap();
        assert!(path.exists());
    });
}

#[test]
fn compile_missing_include_file_errors() {
    with_temp_home(|home| {
        let project = home.join("missing_inc");
        init(&project, false).unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["nonexistent.txt".into()];
        save_config(&project, &config).unwrap();
        let res = compile(&project, None, false, None);
        assert!(matches!(res, Err(ArctgzError::IncludeFileNotFound(_))));
    });
}

#[test]
fn compile_glob_pattern() {
    with_temp_home(|home| {
        let project = home.join("glob_test");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("a.txt"), b"a").unwrap();
        fs::write(project.join("include").join("b.md"), b"b").unwrap();
        let mut cfg = load_config(&project).unwrap();
        cfg.include = vec!["*.txt".into()];
        save_config(&project, &cfg).unwrap();
        let archive = compile(&project, None, false, None).unwrap();
        let f = fs::File::open(&archive).unwrap();
        let gz = flate2::read::GzDecoder::new(f);
        let mut ar = tar::Archive::new(gz);
        let names: Vec<String> = ar
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"a.txt".to_string()));
        assert!(!names.contains(&"b.md".to_string()));
    });
}

#[test]
fn compile_empty_directory() {
    with_temp_home(|home| {
        let project = home.join("empty_dir_test");
        init(&project, false).unwrap();
        fs::create_dir(project.join("include").join("sub")).unwrap();
        let mut cfg = load_config(&project).unwrap();
        cfg.include = vec!["sub".into()];
        save_config(&project, &cfg).unwrap();
        let archive = compile(&project, None, false, None).unwrap();
        let out = home.join("out");
        extract(&archive, &out, false, None).unwrap();
        assert!(out.join("sub").is_dir());
    });
}
