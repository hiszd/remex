#![allow(dead_code)]
use std::io;

use actix::prelude::*;
use actix_codec::{
  Decoder,
  Encoder,
};
use actix_web::web::{
  BufMut,
  BytesMut,
};
use aes_gcm::{
  aead::Aead,
  AeadCore,
  Aes256Gcm,
  KeyInit,
};
use byteorder::{
  BigEndian,
  ByteOrder,
};
use serde::{
  Deserialize,
  Serialize,
};
use serde_json as json;
use surrealdb::types::SurrealValue;
use tracing::error;

#[derive(Serialize, Deserialize, SurrealValue)]
pub struct EndpointSignupCreds {
  pub client_name: String,
  pub hardware_hash: String,
  pub secret: String,
}
#[derive(Serialize, Deserialize, SurrealValue)]
pub struct EndpointSigninCreds {
  pub client_name: String,
  pub client_id: surrealdb::types::RecordId,
  pub hardware_hash: String,
  pub secret: String,
}

// const KEY: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

// FIXME: This should not be hardcoded
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DisconnectReason {
  AuthFailed,
  InvalidClientId,
  DuplicateClient,
  HeartbeatFailed,
  Unknown(String),
}

impl From<DisconnectReason> for String {
  fn from(reason: DisconnectReason) -> String {
    match reason {
      DisconnectReason::AuthFailed => "Authentication failed".to_string(),
      DisconnectReason::InvalidClientId => "Invalid client id".to_string(),
      DisconnectReason::DuplicateClient => "A client with this id is already connected".to_string(),
      DisconnectReason::HeartbeatFailed => {
        "Heartbeat wasn't updated within the proper window".to_string()
      }
      DisconnectReason::Unknown(s) => format!("Unknown: {}", s),
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
      DisconnectReason::DuplicateClient => {
        write!(f, "A client with this id is already connected")
      }
      DisconnectReason::HeartbeatFailed => {
        write!(f, "Heartbeat wasn't updated within the proper window")
      }
      DisconnectReason::Unknown(s) => {
        write!(f, "Unknown: {}", s)
      }
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub enum IdentifyType {
  /// Secret (Secret, Client_Name, Hardware_Hash)
  Secret(String, String, String),
  /// ClientSecret (Secret, Client_Name, Client_Id, Hardware_Hash)
  ClientSecret(String, String, surrealdb::types::RecordId, String),
}

/// Requests related to the connection
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub enum ConnectionRequest {
  /// Identify (Type, Data)
  Identify(IdentifyType),
}

/// Requests related to the connection
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
#[serde(tag = "jobs", content = "data")]
pub enum JobsRequest {
  /// Request all jobs
  All,
  /// Send executions to the server (Job_Id, Executions, Logs)
  SendExecutions(String, (), ()),
  /// Send executions to the server (Job_Id, Executions, Logs)
  UpdateJob(()),
}

/// Client request - come from client
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
#[serde(tag = "cmd", content = "data")]
pub enum ClientRequest {
  /// Connection Request
  ConnectionRequest(ConnectionRequest),
  /// Requesting jobs from the server
  JobsRequest(JobsRequest),
  /// Ping
  Ping,
  /// JwtRefreshAck (jwt_id) - endpoint acknowledges JWT refresh
  JwtRefreshAck { jwt_id: String },
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
pub enum ConnectionResponse {
  /// Authenticated (client_id, jwt)
  Authenticated(surrealdb::types::RecordId, crate::db::BearerGrantResponse),
  /// RefreshJwt (new_jwt, jwt_id) - server pushes new JWT to endpoint
  RefreshJwt { new_jwt: String, jwt_id: String },
  /// Disconnect (Reason)
  Disconnect(DisconnectReason),
}

#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
#[serde(tag = "jobs", content = "data")]
pub enum JobsResponse {
  All(Vec<()>),
  ReceiveJobs(Vec<()>),
}

/// Server response - respond to client requests
#[derive(Serialize, Deserialize, Debug, Message, Clone)]
#[rtype(result = "()")]
#[serde(tag = "cmd", content = "data")]
pub enum ServerResponse {
  /// Connection Response
  ConnectionResponse(ConnectionResponse),
  /// Recieve Jobs from the Session manager
  JobsResponse(JobsResponse),
  /// Ping
  Ping,
}

/// Codec for Client -> Server transport
pub struct ClientCodec;

impl Decoder for ClientCodec {
  type Item = ClientRequest;
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
        Ok(d) => Ok(Some(json::from_slice::<ClientRequest>(d.as_bytes())?)),
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

impl Encoder<ServerResponse> for ClientCodec {
  type Error = io::Error;

  fn encode(&mut self, msg: ServerResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
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
  type Item = ServerResponse;
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
        Ok(d) => Ok(Some(json::from_slice::<ServerResponse>(d.as_bytes())?)),
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

impl Encoder<ClientRequest> for ServerCodec {
  type Error = io::Error;

  fn encode(&mut self, msg: ClientRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
    let msg = json::to_string(&msg).unwrap();
    let m = encrypt(msg);
    let msg_ref: &[u8] = m.as_slice();

    dst.reserve(msg_ref.len() + 2);
    dst.put_u16(msg_ref.len() as u16);
    dst.put(msg_ref);

    Ok(())
  }
}
