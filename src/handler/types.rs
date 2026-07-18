use crate::handler::ArctgzError;
use serde::{Deserialize, Serialize};
use std::path::Component;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArctgzConfig {
    pub name: String,
    pub version: String,
    pub include: Vec<String>,
}

impl Default for ArctgzConfig {
    fn default() -> Self {
        Self {
            name: "untitled".to_string(),
            version: "0.1.0".to_string(),
            include: vec![],
        }
    }
}

impl ArctgzConfig {
    pub fn validate(&self) -> Result<(), ArctgzError> {
        let name = self.name.trim();
        if name.is_empty() {
            return Err(ArctgzError::ConfigValidation(
                "Project name cannot be empty".into(),
            ));
        }
        if name.len() > 255 {
            return Err(ArctgzError::ConfigValidation(
                "Project name must not exceed 255 characters".into(),
            ));
        }
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ArctgzError::ConfigValidation(
                "Project name may only contain letters, digits, '-' and '_'".into(),
            ));
        }

        semver::Version::parse(&self.version).map_err(|e| {
            ArctgzError::ConfigValidation(format!("Invalid version (semver): {}", e))
        })?;

        if self.include.len() > 100 {
            return Err(ArctgzError::ConfigValidation(
                "Maximum of 100 include entries allowed".into(),
            ));
        }

        for (i, inc) in self.include.iter().enumerate() {
            if inc.trim().is_empty() {
                return Err(ArctgzError::ConfigValidation(format!(
                    "Include entry #{} is empty",
                    i + 1
                )));
            }

            let p = std::path::Path::new(inc);
            if p.is_absolute() {
                return Err(ArctgzError::ConfigValidation(format!(
                    "Invalid include '{}': must be a relative path",
                    inc
                )));
            }

            if p.components().any(|c| c == Component::ParentDir) {
                return Err(ArctgzError::ConfigValidation(format!(
                    "Invalid include '{}': must not contain '..' path components",
                    inc
                )));
            }
        }

        Ok(())
    }
}
