use crate::handler::{ArctgzError, ArctgzManifest};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub fn sign_manifest(manifest: &ArctgzManifest, private_key: &[u8]) -> Result<String, ArctgzError> {
    let signing_key = SigningKey::from_bytes(
        private_key
            .try_into()
            .map_err(|_| ArctgzError::KeyError("Private key must be 32 bytes".into()))?,
    );

    let mut manifest_clone = manifest.clone();
    manifest_clone.signature = None;
    let json = serde_json::to_vec(&manifest_clone)?;
    let signature = signing_key.sign(&json);
    Ok(hex::encode(signature.to_bytes()))
}

pub fn verify_manifest(manifest: &ArctgzManifest, public_key: &[u8]) -> Result<(), ArctgzError> {
    let verifying_key = VerifyingKey::from_bytes(
        public_key
            .try_into()
            .map_err(|_| ArctgzError::KeyError("Public key must be 32 bytes".into()))?,
    )
    .map_err(|e| ArctgzError::SignatureError(format!("Invalid public key: {}", e)))?;

    let signature_bytes = hex::decode(manifest.signature.as_deref().unwrap_or(""))
        .map_err(|_| ArctgzError::SignatureError("Invalid hex in signature".into()))?;

    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| ArctgzError::SignatureError(format!("Invalid signature: {}", e)))?;

    let mut manifest_clone = manifest.clone();
    manifest_clone.signature = None;
    let json = serde_json::to_vec(&manifest_clone)?;

    verifying_key
        .verify(&json, &signature)
        .map_err(|_| ArctgzError::SignatureError("Bad signature".into()))
}
