use std::fs;
use std::path::Path;
use tempfile::TempDir;

pub fn with_temp_home<F: FnOnce(&Path)>(test: F) {
    let tmp = TempDir::new().expect("Failed to create base temp dir");
    let home = tmp.path().join("home");
    fs::create_dir(&home).unwrap();

    let home_str = home.to_string_lossy().into_owned();
    let (env_key, env_val) = if cfg!(unix) {
        ("HOME", home_str)
    } else {
        ("USERPROFILE", home_str)
    };

    temp_env::with_var(env_key, Some(&env_val), || test(&home));
}
