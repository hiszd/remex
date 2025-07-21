use actix::prelude::*;
use rand::distr::Alphanumeric;
use rand::Rng;
use tracing::{error, info};

use crate::endpoint::Endpoint;
use crate::messages::session;
use crate::server::Server;
use crate::session::RemexSession;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
  pub identity: Endpoint,
  pub addr: Addr<RemexSession>,
}
/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for Server {
  type Result = ();
  fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
    // TODO: make this try and enroll based on a previous id
    let db = self.db.clone();
    let mut id: Option<String> = None;
    info!("Session connection made with clientname {}", &msg.identity.name);
    match msg.identity.id.is_some() {
      true => {
        self.sessions.insert(msg.identity.clone(), msg.addr.clone()).unwrap();
      }
      false => {
        info!("Creating ID where there was none");
        // generate an id that is 15 characters long of random numbers
        let id =
          rand::rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect::<String>();
        self.sessions.insert(msg.identity.clone(), msg.addr.clone()).unwrap();
        msg.addr.do_send(session::conn::Identified {
          identity: msg.identity.clone(),
          secret: "".to_string(),
          temp: true,
        });
      }
    }
    let server = ctx.address().clone();
    let session = msg.addr.clone();
    let futr = async move {
      db.send(crate::messages::db::NewClient {
        identity: msg.identity.clone(),
        session,
        server,
      })
      .await
      .unwrap()
      .unwrap();
    };
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
  }
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
  pub identity: Endpoint,
}
/// Handler for Disconnect message.
impl Handler<Disconnect> for Server {
  type Result = ();
  fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
    info!("Session {:?} disconnected", &msg.identity);
    // remove address
    self.sessions.remove(msg.identity.id.clone().unwrap());
  }
}

/// Client ID change
#[derive(Message)]
#[rtype(result = "()")]
pub struct IdChange {
  pub old_identity: Endpoint,
  pub new_identity: Endpoint,
  pub addr: Addr<RemexSession>,
}
/// Handler for Identified message.
impl Handler<IdChange> for Server {
  type Result = ();
  fn handle(&mut self, msg: IdChange, _: &mut Context<Self>) -> Self::Result {
    info!("Trying to change id");
    let id = msg.new_identity.clone();
    let old_id = msg.old_identity.clone();
    // TODO: do None check on old_id.id
    if self.sessions.exists(old_id.machineid.clone()) {
      match self.sessions.update_identity(id.machineid.clone(), id.clone()) {
        Ok(_) => info!("Changed id from {:?} to {:?}", &old_id, &id),
        // TODO: maybe remove session if ID cannot be changed?
        Err(e) => error!("Could not change id: {}", e),
      }
    } else {
      error!("Session {:?} does not exist", &old_id);
    }
  }
}

/// DB Client identified
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct DbClientIdentified {
  pub identity: Endpoint,
  pub secret: String,
  pub addr: Addr<RemexSession>,
}
/// Handler for DbClientIDentified message.
///
/// Change ID for session
impl Handler<DbClientIdentified> for Server {
  type Result = ();
  fn handle(&mut self, db: DbClientIdentified, _: &mut Context<Self>) -> Self::Result {
    info!("DB Client identified");
    // info!("Session client being created with id {}", &db.id);
    // match self.sessions.insert(db.id.clone(), db.addr.clone()) {
    //   Err(e) => error!("Could not create session with id: {}", e),
    //   _ => {
    //     db.addr.do_send(session::Identified {
    //       id: db.id.clone(),
    //       name: db.clientname,
    //       secret: db.secret.clone(),
    //       temp: false,
    //     });
    //   }
    // }
  }
}
