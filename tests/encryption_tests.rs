mod common;
use arctgz::{ArctgzError, Encryption, compile, extract, init, load_config, save_config, verify};
use common::with_temp_home;
use std::fs;

#[test]
fn encrypt_and_extract() {
    with_temp_home(|home| {
        let project = home.join("enc_proj");
        init(&project, false).unwrap();
        fs::write(project.join("include").join("secret.txt"), b"top secret").unwrap();
        let mut cfg = load_config(&project).unwrap();
        cfg.include = vec!["secret.txt".into()];
        cfg.encryption = Encryption::Aes256Gcm;
        save_config(&project, &cfg).unwrap();

        let archive = compile(&project, None, false, None, Some("strongpw")).unwrap();
        verify(&archive, None, Some("strongpw")).unwrap();
        let out = home.join("out");
        extract(&archive, &out, false, None, Some("strongpw")).unwrap();
        assert_eq!(
            fs::read_to_string(out.join("secret.txt")).unwrap(),
            "top secret"
        );
    });
}

#[test]
fn wrong_password_fails() {
    with_temp_home(|home| {
        let project = home.join("wrongpw");
        init(&project, false).unwrap();
        let mut cfg = load_config(&project).unwrap();
        cfg.encryption = Encryption::Aes256Gcm;
        save_config(&project, &cfg).unwrap();
        let archive = compile(&project, None, false, None, Some("correct")).unwrap();
        let res = verify(&archive, None, Some("wrong"));
        assert!(matches!(res, Err(ArctgzError::EncryptionError(_))));
    });
}
