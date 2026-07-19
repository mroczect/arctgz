use crate::handler::ArctgzError;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use rand::RngCore;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

const MAGIC: &[u8; 4] = b"ARCT";
const VERSION: u8 = 1;
const SALT_SIZE: usize = 32;
const NONCE_SIZE: usize = 12;

pub fn is_encrypted(path: &Path) -> Result<bool, ArctgzError> {
    let mut file = fs::File::open(path)?;
    let mut magic = [0u8; 4];
    if file.read_exact(&mut magic).is_err() {
        return Ok(false);
    }
    Ok(&magic == MAGIC)
}

pub fn encrypt_file(
    input_path: &Path,
    output_path: &Path,
    password: &str,
) -> Result<(), ArctgzError> {
    let mut salt = [0u8; SALT_SIZE];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| ArctgzError::EncryptionError("Invalid key length".into()))?;
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut plaintext = Vec::new();
    fs::File::open(input_path)?.read_to_end(&mut plaintext)?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|_| ArctgzError::EncryptionError("Encryption failed".into()))?;

    let mut output = fs::File::create(output_path)?;
    output.write_all(MAGIC)?;
    output.write_all(&[VERSION])?;
    output.write_all(&salt)?;
    output.write_all(&nonce_bytes)?;
    output.write_all(&ciphertext)?;
    Ok(())
}

pub fn decrypt_file(
    input_path: &Path,
    output_path: &Path,
    password: &str,
) -> Result<(), ArctgzError> {
    let mut input = fs::File::open(input_path)?;
    let mut magic = [0u8; 4];
    input
        .read_exact(&mut magic)
        .map_err(|_| ArctgzError::EncryptionError("File too short".into()))?;
    if &magic != MAGIC {
        return Err(ArctgzError::EncryptionError(
            "Not an encrypted arctgz file".into(),
        ));
    }
    let mut version = [0u8; 1];
    input.read_exact(&mut version)?;
    let mut salt = [0u8; SALT_SIZE];
    input.read_exact(&mut salt)?;
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    input.read_exact(&mut nonce_bytes)?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let mut ciphertext = Vec::new();
    input.read_to_end(&mut ciphertext)?;

    let key = derive_key(password, &salt)?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| ArctgzError::EncryptionError("Invalid key length".into()))?;
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| ArctgzError::EncryptionError("Decryption failed – wrong password?".into()))?;
    fs::write(output_path, plaintext)?;
    Ok(())
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], ArctgzError> {
    let mut key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|_| ArctgzError::EncryptionError("Key derivation failed".into()))?;
    Ok(key)
}
