use aes::{Aes256, cipher::generic_array::GenericArray};
use base64::{Engine, prelude::BASE64_STANDARD};
use cipher::{BlockEncrypt as _, KeyInit as _};
use intx::U96;
use rand::{TryRngCore as _, rngs::OsRng};
use thiserror::Error;

use crate::CONFIG;

#[derive(Debug, Error)]
pub enum DecryptError {
    #[error("invalid tag")]
    InvalidTag,
    #[error("invalid message")]
    InvalidMessage,
}

// Class for encrypting/decrypting AES-256 GCM with a given key
pub struct Aes256Gcm {
    key: [u8; 32],
}

impl Default for Aes256Gcm {
    fn default() -> Self {
        let mut key = [0u8; 32];
        key.copy_from_slice(&CONFIG.aes_key);

        Self { key }
    }
}

impl Aes256Gcm {
    // Encrypt the given 128-bit block using the AES cipher
    fn aes_encrypt(&self, block: u128) -> u128 {
        let cipher = Aes256::new(GenericArray::from_slice(&self.key));
        let mut bytes = GenericArray::clone_from_slice(&block.to_be_bytes());

        cipher.encrypt_block(&mut bytes);

        u128::from_be_bytes(bytes.into())
    }

    // Returns the hash key, H = E_k(0)
    fn hash_key(&self) -> u128 {
        self.aes_encrypt(0)
    }

    // Increment only the lower 32 bits of a block (wrapping on overflow)
    // ensures upper 96 bits (nonce) don't change
    const fn inc32(block: &mut u128) {
        let mut counter = (*block & 0xFFFF_FFFF) as u32;
        counter = counter.wrapping_add(1);
        let incremented = (*block & !0xFFFF_FFFF) | counter as u128;

        *block = incremented;
    }

    fn pre_counter_block(iv: U96) -> u128 {
        1u128 | (u128::from(iv) << 32)
    }

    // Length of the number-used once in bytes
    const NONCE_LEN: usize = 12;
    // Length of the authentication tag in bytes
    const TAG_LEN: usize = 16;
    
    fn gctr(&self, mut counter: u128, plaintext: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(plaintext.len());

        for chunk in plaintext.chunks(Self::TAG_LEN) {
            Self::inc32(&mut counter);

            let key_stream = self.aes_encrypt(counter).to_be_bytes();
            for i in 0..chunk.len() {
                output.push(chunk[i] ^ key_stream[i]);
            }
        }

        output
    }

    // Performs multiplication of two elements in Galois Field, GF(2^128)
    fn gf_mul(mut a: u128, mut b: u128) -> u128 {
        let modulus: u128 = 0x87;
        let mut result: u128 = 0;

        for _ in 0..128 {
            if (b & 1) != 0 {
                result ^= a;
            }
            let carry = (a >> 127) & 1;
            a <<= 1;
            if carry != 0 {
                a ^= modulus;
            }
            b >>= 1;
        }

        result
    }

    // GHash repeatedly applies:
    // - XOR with data to be authenticated
    // - multiplication with hash key, H, in Galois Field, GF(2^128)
    //
    // This ensures that all the data to be authenticated will contribute towards the final tag
    fn ghash(&self, ciphertext: &[u8]) -> u128 {
        let h = self.hash_key();
        let mut x = 0u128;

        for chunk in ciphertext.chunks(Self::TAG_LEN) {
            let mut bytes = [0u8; Self::TAG_LEN];
            bytes[..chunk.len()].copy_from_slice(chunk);
            let block = u128::from_be_bytes(bytes);

            x = Self::gf_mul(x ^ block, h);
        }

        let length = ciphertext.len() as u128;
        x = Self::gf_mul(x ^ length, h);

        x
    }

    // Encrypt plaintext, with the IV passed in separately,
    // and return both the cipertext and authentication tag explicitly 
    fn encrypt_detached(&self, iv: U96, plaintext: &[u8]) -> (Vec<u8>, u128) {
        let j0 = Self::pre_counter_block(iv);

        let mut counter = j0;
        Self::inc32(&mut counter);

        // encrypt plaintext using AES in Counter Mode
        let ciphertext = self.gctr(counter, plaintext);

        // compute the authneticationt tag
        let s = self.ghash(&ciphertext);
        let tag = s ^ self.aes_encrypt(j0);

        (ciphertext, tag)
    }

    // Generate a random 96-bit integer for the nonce,
    // using the cryptographically-secure RNG provided by the OS
    fn generate_nonce() -> U96 {        
        let mut bytes = [0u8; Self::NONCE_LEN];
        OsRng.try_fill_bytes(&mut bytes).unwrap();

        U96::from_be_bytes(bytes)
    }

    // Encrypt the plaintext into ciphertext, both as a string of bytes
    pub fn encrypt(plaintext: &[u8]) -> Vec<u8> {
        // create a single buffer of bytes split into nonce, ciphertext, and tag
        let mut out = vec![0u8; Self::NONCE_LEN + plaintext.len() + Self::TAG_LEN];

        let (nonce_slice, remains) = out.split_at_mut(Self::NONCE_LEN);
        let (cipher_slice, tag_slice) = remains.split_at_mut(plaintext.len());

        let nonce = Self::generate_nonce();
        nonce_slice.copy_from_slice(&nonce.to_be_bytes());

        let (ciphertext, tag) = Self::default().encrypt_detached(nonce, plaintext);

        cipher_slice.copy_from_slice(&ciphertext);
        tag_slice.copy_from_slice(&tag.to_be_bytes());

        out
    }

    // Encrypt the plaintext, outputting the ciphertext using Base64 encoding
    pub fn encrypt_base64(plaintext: &[u8]) -> String {
        BASE64_STANDARD.encode(Self::encrypt(plaintext))
    }

    // Decrypt the ciphertext with the IV an tag passed in separately
    fn decrypt_detached(
        &self,
        iv: U96,
        ciphertext: &[u8],
        tag: u128,
    ) -> Result<Vec<u8>, DecryptError> {
        let j0 = Self::pre_counter_block(iv);

        // calculate the expected tag
        let s = self.ghash(ciphertext);
        let expected_tag = s ^ self.aes_encrypt(j0);

        // If the actual tag doesn't match the calculated expected tag,
        // then the data has been corrupted during transmission
        if expected_tag != tag {
            return Err(DecryptError::InvalidTag);
        }

        let mut counter = j0;
        Self::inc32(&mut counter);

        let plaintext = self.gctr(counter, ciphertext);

        Ok(plaintext)
    }

    // Decrypt the ciphertext bytes (incl. IV and tag) into plaintext
    pub fn decrypt(value: &[u8]) -> Result<Vec<u8>, DecryptError> {
        // split the inputted bytes into IV, ciphertext, and tag
        let (nonce, remaining) = value
            .split_at_checked(Self::NONCE_LEN)
            .ok_or(DecryptError::InvalidMessage)?;

        let tag_pos = remaining.len() - Self::TAG_LEN;
        let (ciphertext, tag) = remaining
            .split_at_checked(tag_pos)
            .ok_or(DecryptError::InvalidMessage)?;

        let nonce = U96::from_be_bytes(nonce.try_into().unwrap());
        let tag = u128::from_be_bytes(tag.try_into().unwrap());

        Self::default().decrypt_detached(nonce, ciphertext, tag)
    }
}
