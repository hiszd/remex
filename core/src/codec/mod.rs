#![allow(dead_code)]
use std::io;

use actix_codec::{Decoder, Encoder};
use actix_web::web::{BufMut, BytesMut};
use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit};
use byteorder::{BigEndian, ByteOrder};
use serde::{Deserialize, Serialize};
use serde_json as json;
use tracing::error;

pub mod c2s;
pub mod s2c;

// const KEY: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";
const KEY: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZn";

fn decrypt(encrypted_data: Vec<u8>) -> Result<String, String> {
  let key = aes_gcm::Key::<Aes256Gcm>::from_slice(KEY.as_bytes());
  let (nonce_arr, ciphered_data) = encrypted_data.split_at(12);
  let nonce = aes_gcm::Nonce::from_slice(nonce_arr);
  let cipher = Aes256Gcm::new(key);
  match cipher.decrypt(nonce, ciphered_data) {
    Ok(plaintext) => {
      let pt = String::from_utf8(plaintext);
      match pt {
        Ok(p) => Ok(p),
        Err(_) => Err("failed to convert from utf8".to_string()),
      }
    }
    Err(_) => Err("Failed to decrypt".to_string()),
  }
}

fn encrypt(plaintext: String) -> Vec<u8> {
  let key = aes_gcm::Key::<Aes256Gcm>::from_slice(KEY.as_bytes());
  let nonce = Aes256Gcm::generate_nonce(&mut aes_gcm::aead::OsRng);
  let cipher = Aes256Gcm::new(key);
  let ciphered_data = cipher
    .encrypt(&nonce, plaintext.as_bytes())
    .expect("failed to encrypt");
  // combining nonce and encrypted data together
  // for storage purpose
  let mut encrypted_data: Vec<u8> = nonce.to_vec();
  encrypted_data.extend_from_slice(&ciphered_data);
  encrypted_data
}

/// Enum that describes the authentication request from the client
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthRequest {
  /// Client id and secret (Id, Secret)
  IdSecret(String, String),
  /// Shared secret (Secret)
  Secret(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DisconnectReason {
  AuthFailed,
  InvalidClientId,
  InvalidSecret,
}
impl From<DisconnectReason> for String {
  fn from(val: DisconnectReason) -> Self {
    match val {
      DisconnectReason::AuthFailed => "Authentication failed".to_string(),
      DisconnectReason::InvalidClientId => "Invalid client id".to_string(),
      DisconnectReason::InvalidSecret => "Invalid secret".to_string(),
    }
  }
}
impl std::fmt::Display for DisconnectReason {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      DisconnectReason::AuthFailed => {
        write!(f, "Authentication failed")
      }
      DisconnectReason::InvalidClientId => {
        write!(f, "Invalid client id")
      }
      DisconnectReason::InvalidSecret => {
        write!(f, "Invalid secret")
      }
    }
  }
}

/// Codec for Client -> Server transport
pub struct ClientCodec;

impl Decoder for ClientCodec {
  type Item = c2s::C2S;
  type Error = io::Error;

  fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
    let size = {
      if src.len() < 2 {
        return Ok(None);
      }
      BigEndian::read_u16(src.as_ref()) as usize
    };

    if src.len() >= size + 2 {
      let _ = src.split_to(2);
      let buf = src.split_to(size);
      let dcpt = decrypt(buf.to_vec());
      match dcpt {
        Ok(d) => Ok(Some(json::from_slice::<c2s::C2S>(d.as_bytes())?)),
        Err(e) => {
          if e == "Failed to decrypt" {
            error!("Failed to decrypt, maybe the key is wrong");
          }
          Err(io::Error::other(e))
        }
      }
    } else {
      Ok(None)
    }
  }
}

impl Encoder<s2c::S2C> for ClientCodec {
  type Error = io::Error;

  fn encode(&mut self, msg: s2c::S2C, dst: &mut BytesMut) -> Result<(), Self::Error> {
    let msg = json::to_string(&msg).unwrap();
    let m = encrypt(msg);
    let msg_ref: &[u8] = m.as_slice();

    dst.reserve(msg_ref.len() + 2);
    dst.put_u16(msg_ref.len() as u16);
    dst.put(msg_ref);

    Ok(())
  }
}

/// Codec for Server -> Client transport
pub struct ServerCodec;

impl Decoder for ServerCodec {
  type Item = s2c::S2C;
  type Error = io::Error;

  fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
    let size = {
      if src.len() < 2 {
        return Ok(None);
      }
      BigEndian::read_u16(src.as_ref()) as usize
    };

    if src.len() >= size + 2 {
      let _ = src.split_to(2);
      let buf = src.split_to(size);
      let dcpt = decrypt(buf.to_vec());
      match dcpt {
        Ok(d) => Ok(Some(json::from_slice::<s2c::S2C>(d.as_bytes())?)),
        Err(e) => {
          if e == "Failed to decrypt" {
            error!("Failed to decrypt, maybe the key is wrong");
          }
          Err(io::Error::other(e))
        }
      }
    } else {
      Ok(None)
    }
  }
}

impl Encoder<c2s::C2S> for ServerCodec {
  type Error = io::Error;

  fn encode(&mut self, msg: c2s::C2S, dst: &mut BytesMut) -> Result<(), Self::Error> {
    let msg = json::to_string(&msg).unwrap();
    let m = encrypt(msg);
    let msg_ref: &[u8] = m.as_slice();

    dst.reserve(msg_ref.len() + 2);
    dst.put_u16(msg_ref.len() as u16);
    dst.put(msg_ref);

    Ok(())
  }
}
