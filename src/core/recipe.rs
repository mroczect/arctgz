use crate::handler::{ArctgzError, ArctgzRecipe, RecipeStep};
use std::fs;
use std::fs::File;
use std::io::Read;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path};

pub fn load_recipe(project_path: &Path) -> Result<Option<ArctgzRecipe>, ArctgzError> {
    let recipe_path = project_path.join("recipe.json");
    if !recipe_path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&recipe_path)?;
    let recipe: ArctgzRecipe = serde_json::from_str(&data)?;
    validate_recipe(&recipe)?;
    Ok(Some(recipe))
}

pub fn extract_recipe(archive_path: &Path) -> Result<ArctgzRecipe, ArctgzError> {
    let (_, compression) = crate::core::archive::read_manifest(archive_path)?;

    let file = File::open(archive_path)?;
    let decoder = crate::core::archive::make_reader_from_file(file, &compression)?;
    let mut archive = tar::Archive::new(decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();

        if path == "manifest.json" {
            let mut sink = [0u8; 8192];
            while entry.read(&mut sink)? > 0 {}
            continue;
        }

        if path == "recipe.json" {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            let recipe: ArctgzRecipe = serde_json::from_slice(&buf)?;
            validate_recipe(&recipe)?;
            return Ok(recipe);
        }
    }
    Err(ArctgzError::RecipeNotFound)
}

pub fn execute_recipe(
    output_dir: &Path,
    recipe: &ArctgzRecipe,
    force: bool,
) -> Result<(), ArctgzError> {
    validate_recipe(recipe)?;

    for step in &recipe.steps {
        match step {
            RecipeStep::Copy { from, to } => {
                let src = output_dir.join(from);
                let dst = output_dir.join(to);
                if dst.exists() && !force {
                    return Err(ArctgzError::RecipeExecutionError(format!(
                        "destination already exists: {}",
                        dst.display()
                    )));
                }
                if src.is_dir() {
                    copy_dir_recursively(&src, &dst)?;
                } else {
                    if let Some(parent) = dst.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(&src, &dst)?;
                }
            }
            RecipeStep::MkDir { path } => {
                let dir = output_dir.join(path);
                if dir.exists() && !force {
                    return Err(ArctgzError::RecipeExecutionError(format!(
                        "directory already exists: {}",
                        dir.display()
                    )));
                }
                fs::create_dir_all(&dir)?;
            }
            RecipeStep::Chmod { path, mode } => {
                #[cfg(not(unix))]
                return Err(ArctgzError::RecipeExecutionError(
                    "chmod is not supported on this platform".into(),
                ));
                #[cfg(unix)]
                {
                    let target = output_dir.join(path);
                    let mode = u32::from_str_radix(mode, 8).map_err(|_| {
                        ArctgzError::RecipeExecutionError(format!("invalid mode: {}", mode))
                    })?;
                    let mut perms = fs::metadata(&target)?.permissions();
                    perms.set_mode(mode);
                    fs::set_permissions(&target, perms)?;
                }
            }
            RecipeStep::Remove { path } => {
                let target = output_dir.join(path);
                if !target.exists() {
                    return Err(ArctgzError::RecipeExecutionError(format!(
                        "path does not exist: {}",
                        target.display()
                    )));
                }
                if target.is_dir() {
                    fs::remove_dir(&target)?;
                } else {
                    fs::remove_file(&target)?;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_recipe(recipe: &ArctgzRecipe) -> Result<(), ArctgzError> {
    for (i, step) in recipe.steps.iter().enumerate() {
        match step {
            RecipeStep::Copy { from, to } => {
                validate_safe_path(from)?;
                validate_safe_path(to)?;
            }
            RecipeStep::MkDir { path } => {
                validate_safe_path(path)?;
            }
            RecipeStep::Chmod { path, mode } => {
                validate_safe_path(path)?;
                u32::from_str_radix(mode, 8).map_err(|_| {
                    ArctgzError::RecipeInvalid(format!("step {}: invalid mode '{}'", i + 1, mode))
                })?;
            }
            RecipeStep::Remove { path } => {
                validate_safe_path(path)?;
            }
        }
    }
    Ok(())
}

fn validate_safe_path(path: &str) -> Result<(), ArctgzError> {
    if path.is_empty() || path == "." {
        return Err(ArctgzError::RecipeInvalid(format!(
            "unsafe path (points to base directory): '{}'",
            path
        )));
    }

    let p = Path::new(path);
    if p.is_absolute() || p.components().any(|c| c == Component::ParentDir) {
        return Err(ArctgzError::RecipeInvalid(format!("unsafe path: {}", path)));
    }
    Ok(())
}

fn copy_dir_recursively(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursively(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
