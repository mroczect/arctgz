mod common;

use arctgz::{ArctgzError, init};
use common::with_temp_home;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn init_creates_include_and_config() {
    with_temp_home(|home| {
        let project = home.join("my_project");
        init(&project, false).expect("Init should succeed");

        assert!(project.join("include").is_dir());
        let config_path = project.join("arctgz.init");
        assert!(config_path.is_file());

        let content = fs::read_to_string(&config_path).unwrap();
        let expected = serde_json::json!({
            "name": "untitled",
            "version": "0.1.0",
            "include": [],
            "compression": "gzip"
        });
        let actual: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(actual, expected);
    });
}

#[test]
fn init_empty_existing_directory() {
    with_temp_home(|home| {
        let project = home.join("empty_dir");
        fs::create_dir(&project).unwrap();
        init(&project, false).expect("Init on empty dir should succeed");
        assert!(project.join("arctgz.init").exists());
    });
}

#[test]
fn init_nonempty_force_succeeds() {
    with_temp_home(|home| {
        let project = home.join("nonempty_force");
        fs::create_dir(&project).unwrap();
        File::create(project.join("existing.txt")).unwrap();
        init(&project, true).expect("Force init should succeed");
        assert!(project.join("arctgz.init").exists());
    });
}

#[test]
fn init_nonempty_no_force_errors() {
    with_temp_home(|home| {
        let project = home.join("nonempty_err");
        fs::create_dir(&project).unwrap();
        File::create(project.join("somefile")).unwrap();
        let res = init(&project, false);
        assert!(matches!(res, Err(ArctgzError::DirectoryNotEmpty(_))));
    });
}

#[test]
fn init_empty_path_errors() {
    with_temp_home(|_home| {
        let res = init(Path::new(""), false);
        assert!(matches!(res, Err(ArctgzError::InvalidPath(_))));
    });
}

#[test]
fn init_outside_home_existing_errors() {
    let tmp = TempDir::new().unwrap();
    let outside = tmp.path().join("outside_dir");
    fs::create_dir(&outside).unwrap();
    let home = tmp.path().join("home");
    fs::create_dir(&home).unwrap();

    let (env_key, env_val) = if cfg!(unix) {
        ("HOME", home.to_str().unwrap())
    } else {
        ("USERPROFILE", home.to_str().unwrap())
    };

    temp_env::with_var(env_key, Some(env_val), || {
        let res = init(&outside, false);
        assert!(matches!(res, Err(ArctgzError::PathNotAllowed(_))));
    });
}

#[test]
fn init_outside_home_nonexistent_no_side_effect() {
    let tmp = TempDir::new().unwrap();
    let outside_new = tmp.path().join("outside_new");
    let home = tmp.path().join("home");
    fs::create_dir(&home).unwrap();

    let (env_key, env_val) = if cfg!(unix) {
        ("HOME", home.to_str().unwrap())
    } else {
        ("USERPROFILE", home.to_str().unwrap())
    };

    temp_env::with_var(env_key, Some(env_val), || {
        let res = init(&outside_new, false);
        assert!(matches!(res, Err(ArctgzError::PathNotAllowed(_))));
        assert!(!outside_new.exists());
    });
}

#[test]
fn init_path_is_file_errors() {
    with_temp_home(|home| {
        let file_path = home.join("not_a_dir");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"data").unwrap();
        let res = init(&file_path, false);
        assert!(matches!(res, Err(ArctgzError::InvalidPath(_))));
    });
}

#[test]
fn init_twice_no_force_errors_directory_not_empty() {
    with_temp_home(|home| {
        let project = home.join("twice_no_force");
        init(&project, false).unwrap();
        let res = init(&project, false);
        assert!(matches!(res, Err(ArctgzError::DirectoryNotEmpty(_))));
    });
}

#[test]
fn init_twice_with_force_errors_already_initialized() {
    with_temp_home(|home| {
        let project = home.join("twice_with_force");
        init(&project, false).unwrap();
        let res = init(&project, true);
        assert!(matches!(res, Err(ArctgzError::AlreadyInitialized)));
    });
}

#[cfg(unix)]
#[test]
fn init_symlink_outside_home_errors() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir(&home).unwrap();
    let outside_target = tmp.path().join("outside_target");
    fs::create_dir(&outside_target).unwrap();
    let symlink = home.join("bad_link");
    std::os::unix::fs::symlink(&outside_target, &symlink).unwrap();

    temp_env::with_var("HOME", Some(home.to_str().unwrap()), || {
        let res = init(&symlink, false);
        assert!(matches!(res, Err(ArctgzError::PathNotAllowed(_))));
    });
}

#[test]
fn init_unicode_path() {
    with_temp_home(|home| {
        let project = home.join("プロジェクト");
        init(&project, false).expect("Unicode path should work");
        assert!(project.join("arctgz.init").exists());
        assert!(project.join("include").is_dir());
    });
}

#[test]
fn init_path_with_spaces() {
    with_temp_home(|home| {
        let project = home.join("my project (v1)");
        init(&project, false).expect("Path with spaces should work");
        assert!(project.join("arctgz.init").exists());
    });
}

#[test]
fn default_config_is_valid() {
    let config = arctgz::ArctgzConfig::default();
    config.validate().expect("Default config must be valid");
}
