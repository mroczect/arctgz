use crate::handler::{ArctgzConfig, ArctgzError};
use std::fs;
use std::path::Path;

pub fn load_config(project_path: &Path) -> Result<ArctgzConfig, ArctgzError> {
    let config_path = project_path.join("arctgz.init");
    if !config_path.exists() {
        return Err(ArctgzError::ConfigNotFound(format!(
            "{}",
            config_path.display()
        )));
    }

    let content = fs::read_to_string(&config_path).map_err(|e| {
        ArctgzError::ConfigLoadError(format!("Failed to read {}: {}", config_path.display(), e))
    })?;

    let config: ArctgzConfig = serde_json::from_str(&content).map_err(|e| {
        ArctgzError::ConfigLoadError(format!("Invalid JSON in {}: {}", config_path.display(), e))
    })?;

    config.validate()?;
    Ok(config)
}

pub fn save_config(project_path: &Path, config: &ArctgzConfig) -> Result<(), ArctgzError> {
    config.validate()?;

    let config_path = project_path.join("arctgz.init");
    let json = serde_json::to_string_pretty(&config)?;

    let temp_path = config_path.with_extension("tmp");
    fs::write(&temp_path, &json).map_err(|e| {
        ArctgzError::ConfigSaveError(format!("Failed to write temporary config: {}", e))
    })?;

    fs::rename(&temp_path, &config_path).map_err(|e| {
        let _ = fs::remove_file(&temp_path);
        ArctgzError::ConfigSaveError(format!("Failed to finalize config: {}", e))
    })?;

    Ok(())
}
