use aes_gcm::{aead::{generic_array::GenericArray, Aead, AeadMutInPlace, OsRng}, AeadCore, Aes256Gcm, Key, KeyInit};

use crate::CONFIG;

// Aes256Gcm uses 96 bit (12 byte) nonces and 128 bit (16 byte) tags
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

fn key() -> &'static Key<Aes256Gcm> {
    Key::<Aes256Gcm>::from_slice(&CONFIG.aes_key)
}

pub fn encrypt(value: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; NONCE_LEN + value.len() + TAG_LEN];

    let (nonce, remains) = out.split_at_mut(NONCE_LEN);
    let (buffer, tag) = remains.split_at_mut(value.len());
    buffer.copy_from_slice(value);

    let generated_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    nonce.copy_from_slice(&generated_nonce);

    let mut cipher = Aes256Gcm::new(key());
    let cipher_tag = cipher.encrypt_in_place_detached(GenericArray::from_slice(nonce), b"", buffer).expect("failed to encrypt");
    tag.copy_from_slice(&cipher_tag);

    out
}

pub fn decrypt(value: &[u8]) -> Result<Vec<u8>, aes_gcm::Error> {
    let (nonce, ciphertext) = value.split_at(NONCE_LEN);

    let cipher = Aes256Gcm::new(key());
    cipher.decrypt(GenericArray::from_slice(nonce), ciphertext)
}