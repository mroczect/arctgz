use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub fn with_temp_home<F: FnOnce(&Path)>(test: F) {
    let tmp = TempDir::new().expect("Failed to create base temp dir");
    let home = tmp.path().join("home");
    fs::create_dir(&home).unwrap();

    let (env_key, env_val) = if cfg!(unix) {
        ("HOME", home.to_str().unwrap().to_owned())
    } else {
        ("USERPROFILE", home.to_str().unwrap().to_owned())
    };

    temp_env::with_var(env_key, Some(&env_val), || test(&home));
}
