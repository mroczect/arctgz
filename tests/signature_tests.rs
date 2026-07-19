mod common;
use common::with_temp_home;

use arctgz::{ArctgzError, compile, extract, init, load_config, save_config, verify};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::fs;

#[test]
fn sign_and_verify() {
    with_temp_home(|home| {
        let project = home.join("signed_proj");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("data.txt"), b"secret").unwrap();
        let mut config = load_config(&project).unwrap();
        config.include = vec!["data.txt".into()];
        save_config(&project, &config).unwrap();

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let archive = compile(&project, None, false, Some(signing_key.as_bytes()), None).unwrap();

        verify(&archive, Some(verifying_key.as_bytes()), None).unwrap();

        let out = home.join("out");
        extract(&archive, &out, false, Some(verifying_key.as_bytes()), None).unwrap();
        assert_eq!(fs::read_to_string(out.join("data.txt")).unwrap(), "secret");
    });
}

#[test]
fn sign_and_verify_wrong_key_fails() {
    with_temp_home(|home| {
        let project = home.join("wrong_key");
        init(&project, false).unwrap();
        let config = load_config(&project).unwrap();
        save_config(&project, &config).unwrap();

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let wrong_key = SigningKey::generate(&mut csprng).verifying_key();

        let archive = compile(&project, None, false, Some(signing_key.as_bytes()), None).unwrap();

        let res = verify(&archive, Some(wrong_key.as_bytes()), None);
        assert!(matches!(res, Err(ArctgzError::SignatureError(_))));
    });
}

#[test]
fn unsigned_archive_verify_with_public_key_fails() {
    with_temp_home(|home| {
        let project = home.join("unsigned");
        init(&project, false).unwrap();
        let archive = compile(&project, None, false, None, None).unwrap();

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let vk = signing_key.verifying_key();

        let res = verify(&archive, Some(vk.as_bytes()), None);
        assert!(matches!(res, Err(ArctgzError::SignatureError(_))));
    });
}
