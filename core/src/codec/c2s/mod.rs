use actix::Message;
use serde::{
  Deserialize,
  Serialize,
};

use crate::endpoint::Endpoint;

// CHANGEPOINT: If you need to add more message categories, add them here. Reference the Conn enum
// for an example of how to do it

/// Client request - requests for connection related messages
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "conn", content = "data")]
pub enum Conn {
  /// Command (Command)
  Command(String),
  /// Try to allow connection with the server based on the ID that was saved on the client or the
  /// secret
  /// Identify (Identity, Secret)
  Identify(Endpoint, super::AuthRequest),
  /// Log (Message)
  Log(String),
  /// Result (Req, Result)
  Result(Box<Conn>, Result<String, String>),
  /// Message (Message)
  Message(String),
  /// Ping
  Ping,
}

/// Client request -
#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "exc", content = "data")]
pub enum Exchange {
  SendConfiguration,
}

/// All messages from the client to the server
// CHANGEPOINT: This is where you add the client messages to the server.
// If these requests fit into one of the categories, add them to the appropriate enum
#[derive(Serialize, Deserialize, Debug, Message)]
#[rtype(result = "()")]
#[serde(tag = "cmd", content = "data")]
pub enum C2S {
  /// Connection related messages
  Conn(Conn),
  Exchange(Exchange),
}
