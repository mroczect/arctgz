mod common;

use arctgz::{ArctgzError, init, load_config, save_config};
use common::with_temp_home;
use std::fs;

#[test]
fn load_config_after_init_returns_default() {
    with_temp_home(|home| {
        let project = home.join("load_default");
        init(&project, false).unwrap();
        let config = load_config(&project).unwrap();
        assert_eq!(config.name, "untitled");
        assert_eq!(config.version, "0.1.0");
        assert!(config.include.is_empty());
    });
}

#[test]
fn save_and_reload_preserves_changes() {
    with_temp_home(|home| {
        let project = home.join("save_reload");
        init(&project, false).unwrap();

        let mut config = load_config(&project).unwrap();
        config.name = "my-app".into();
        config.version = "2.0.0".into();
        config.include = vec!["assets".into(), "lib".into()];
        save_config(&project, &config).unwrap();

        let reloaded = load_config(&project).unwrap();
        assert_eq!(reloaded.name, "my-app");
        assert_eq!(reloaded.version, "2.0.0");
        assert_eq!(reloaded.include, vec!["assets", "lib"]);
    });
}

#[test]
fn load_config_missing_file_errors() {
    with_temp_home(|home| {
        let project = home.join("no_config");
        fs::create_dir(&project).unwrap();
        let res = load_config(&project);
        assert!(matches!(res, Err(ArctgzError::ConfigNotFound(_))));
    });
}

#[test]
fn load_config_invalid_json_errors() {
    with_temp_home(|home| {
        let project = home.join("bad_json");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("arctgz.init"), b"not json").unwrap();
        let res = load_config(&project);
        assert!(matches!(res, Err(ArctgzError::ConfigLoadError(_))));
    });
}

#[test]
fn save_config_invalid_data_errors() {
    with_temp_home(|home| {
        let project = home.join("invalid_save");
        init(&project, false).unwrap();
        let mut config = load_config(&project).unwrap();
        config.name = "bad/name".into();
        let res = save_config(&project, &config);
        assert!(matches!(res, Err(ArctgzError::ConfigValidation(_))));
    });
}

#[test]
fn save_config_is_atomic() {
    with_temp_home(|home| {
        let project = home.join("atomic");
        init(&project, false).unwrap();
        let config = load_config(&project).unwrap();
        save_config(&project, &config).unwrap();
        assert!(!project.join("arctgz.tmp").exists());
        assert!(project.join("arctgz.init").exists());
    });
}
