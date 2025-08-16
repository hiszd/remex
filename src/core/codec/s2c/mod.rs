use actix::Message;
use serde::{Deserialize, Serialize};

use crate::endpoint::executor::Executor;
use crate::endpoint::Endpoint;

// CHANGEPOINT: If you need to add more message categories, add them here. Reference the Conn enum
// for an example of how to do it

/// Server response - respond to connection related messages
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "conn", content = "data")]
pub enum Conn {
  /// Command (Command)
  Command(String),
  /// Message (Message)
  Message(String),
  /// Request the client to identify itself using a saved ID, or using a secret
  /// Identify
  Identify,
  /// Authenticated (Endpoint, Secret)
  Authenticated(Endpoint, String),
  /// Result (Req, Result)
  Result(Box<Conn>, Result<String, String>),
  /// Disconnect (Reason)
  Disconnect(super::DisconnectReason),
  /// Ping
  Ping,
}

/// Server response -
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "exc", content = "data")]
pub enum Exchange {
  ExecutorList(Vec<Executor>),
}

/// All messages from the server to the client
// CHANGEPOINT: This is where you add the server messages to the client.
// If the new request fits into one of the categories, add it to the appropriate enum instead of
// the parent here
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "cmd", content = "data")]
pub enum S2C {
  /// Connection related messages
  Conn(Conn),
  Exchange(Exchange),
}
