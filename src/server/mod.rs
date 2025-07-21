//! `RemexServer` is an actor. It maintains list of connection client session.
//! And manages available rooms. Peers send messages to other peers in same
//! room through `RemexServer`.

use actix::prelude::*;

use crate::sessionmap::SessionMap;

/// `RemexServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct Server {
  pub sessions: SessionMap,
  pub db: Addr<crate::db::Db>,
}
/// Make actor from `RemexServer`
impl Actor for Server {
  /// We are going to use simple Context, we just need ability to communicate
  /// with other actors.
  type Context = Context<Self>;
}
