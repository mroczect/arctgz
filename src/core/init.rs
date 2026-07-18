use crate::handler::{ArctgzConfig, ArctgzError};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

pub fn init(project_path: &Path, force: bool) -> Result<(), ArctgzError> {
    if project_path.as_os_str().is_empty() {
        return Err(ArctgzError::InvalidPath(
            "Project path cannot be empty".into(),
        ));
    }

    let home = dirs::home_dir()
        .ok_or_else(|| ArctgzError::PathNotAllowed("Could not determine home directory".into()))?;
    let canonical_home = fs::canonicalize(&home)?;

    let canonical = if project_path.exists() {
        fs::canonicalize(project_path)?
    } else {
        let ancestor = project_path
            .ancestors()
            .find(|a| a.exists())
            .unwrap_or_else(|| Path::new("."));
        let canonical_ancestor = fs::canonicalize(ancestor)?;
        let remainder = project_path.strip_prefix(ancestor).unwrap_or(Path::new(""));
        canonical_ancestor.join(remainder)
    };

    if !canonical.starts_with(&canonical_home) {
        return Err(ArctgzError::PathNotAllowed(format!(
            "Initialization only allowed under home directory ({}). \
             The resolved path would be {}",
            canonical_home.display(),
            canonical.display()
        )));
    }

    if canonical.exists() && !canonical.is_dir() {
        return Err(ArctgzError::InvalidPath(
            "Path already exists but is not a directory".into(),
        ));
    }

    if !force && canonical.exists() {
        let mut entries = fs::read_dir(&canonical)?;
        if entries.next().is_some() {
            return Err(ArctgzError::DirectoryNotEmpty(
                "Project directory is not empty. Use force = true to overwrite.".into(),
            ));
        }
    }

    fs::create_dir_all(&canonical)?;
    let include_dir = canonical.join("include");
    fs::create_dir_all(&include_dir)?;

    let config = ArctgzConfig::default();
    config.validate()?;

    let json = serde_json::to_string_pretty(&config)?;

    let config_path = canonical.join("arctgz.init");
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config_path)
    {
        Ok(mut file) => {
            file.write_all(json.as_bytes())?;
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err(ArctgzError::AlreadyInitialized);
        }
        Err(e) => return Err(ArctgzError::Io(e)),
    }

    Ok(())
}
