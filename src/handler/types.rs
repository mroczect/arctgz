use crate::handler::ArctgzError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Component;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArctgzConfig {
    pub name: String,
    pub version: String,
    pub include: Vec<String>,
    #[serde(default = "default_compression")]
    pub compression: Compression,
}

fn default_compression() -> Compression {
    Compression::Gzip
}

impl Default for ArctgzConfig {
    fn default() -> Self {
        Self {
            name: "untitled".into(),
            version: "0.1.0".into(),
            include: vec![],
            compression: Compression::Gzip,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArctgzManifest {
    pub name: String,
    pub version: String,
    pub created: DateTime<Utc>,
    #[serde(default)]
    pub compression: Compression,
    pub files: std::collections::BTreeMap<String, FileEntry>,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub size: u64,
    pub sha512: String,
    #[serde(default)]
    pub is_dir: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArctgzRecipe {
    pub name: String,
    pub version: String,
    pub steps: Vec<RecipeStep>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "action")]
pub enum RecipeStep {
    #[serde(rename = "copy")]
    Copy { from: String, to: String },
    #[serde(rename = "mkdir")]
    MkDir { path: String },
    #[serde(rename = "chmod")]
    Chmod { path: String, mode: String },
    #[serde(rename = "remove")]
    Remove { path: String },
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Default)]
pub enum Compression {
    #[serde(rename = "gzip")]
    #[default]
    Gzip,
    #[serde(rename = "zstd")]
    Zstd,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArctgzDelta {
    pub base_name: String,
    pub base_version: String,
    pub target_name: String,
    pub target_version: String,
    pub base_manifest_hash: String,
    pub target_manifest_hash: String,
    pub operations: Vec<DeltaOp>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "op")]
pub enum DeltaOp {
    #[serde(rename = "add")]
    Add {
        path: String,
        size: u64,
        sha512: String,
        #[serde(default)]
        is_dir: bool,
    },
    #[serde(rename = "modify")]
    Modify {
        path: String,
        size: u64,
        sha512: String,
        #[serde(default)]
        is_dir: bool,
    },
    #[serde(rename = "delete")]
    Delete { path: String },
}
