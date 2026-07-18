mod common;
use common::with_temp_home;

use arctgz::{
    ArctgzError, compile, execute_recipe, extract, extract_recipe, init, load_config, save_config,
};
use std::fs;

#[test]
fn execute_recipe_copies_files() {
    with_temp_home(|home| {
        let project = home.join("recipe_proj");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("data.txt"), b"data").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["data.txt".into()];
        save_config(&project, &config).unwrap();

        let recipe = serde_json::json!({
            "name": "test-recipe",
            "version": "0.1.0",
            "steps": [
                { "action": "copy", "from": "data.txt", "to": "etc/data_copy.txt" },
                { "action": "mkdir", "path": "var/log" }
            ]
        });
        fs::write(
            project.join("recipe.json"),
            serde_json::to_string_pretty(&recipe).unwrap(),
        )
        .unwrap();

        let archive = compile(&project, None, false).unwrap();
        let out_dir = home.join("out");
        extract(&archive, &out_dir, false).unwrap();
        let r = extract_recipe(&archive).unwrap();
        execute_recipe(&out_dir, &r, false).unwrap();

        assert!(out_dir.join("etc/data_copy.txt").exists());
        assert!(out_dir.join("var/log").is_dir());
        assert!(out_dir.join("data.txt").exists());
    });
}

#[test]
fn execute_recipe_with_force_overwrites() {
    with_temp_home(|home| {
        let project = home.join("force_recipe");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("f.txt"), b"new").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["f.txt".into()];
        save_config(&project, &config).unwrap();

        let recipe = serde_json::json!({
            "name": "test-force",
            "version": "0.1.0",
            "steps": [
                { "action": "copy", "from": "f.txt", "to": "dest.txt" }
            ]
        });
        fs::write(
            project.join("recipe.json"),
            serde_json::to_string_pretty(&recipe).unwrap(),
        )
        .unwrap();

        let archive = compile(&project, None, false).unwrap();
        let out_dir = home.join("out");
        extract(&archive, &out_dir, false).unwrap();
        fs::write(out_dir.join("dest.txt"), b"old").unwrap();
        let r = extract_recipe(&archive).unwrap();
        assert!(execute_recipe(&out_dir, &r, false).is_err());
        execute_recipe(&out_dir, &r, true).unwrap();
        assert_eq!(fs::read_to_string(out_dir.join("dest.txt")).unwrap(), "new");
    });
}

#[test]
fn recipe_path_traversal_rejected() {
    with_temp_home(|home| {
        let project = home.join("traversal");
        init(&project, false).unwrap();
        let recipe = serde_json::json!({
            "name": "test-traversal",
            "version": "0.1.0",
            "steps": [
                { "action": "copy", "from": "file.txt", "to": "../../escape" }
            ]
        });
        fs::write(
            project.join("recipe.json"),
            serde_json::to_string_pretty(&recipe).unwrap(),
        )
        .unwrap();
        let res = compile(&project, None, false);
        assert!(matches!(res, Err(ArctgzError::RecipeInvalid(_))));
    });
}
