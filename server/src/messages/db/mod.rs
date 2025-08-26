use actix::{
  Context,
  Handler,
  Message,
};

pub mod conn;
pub mod exchange;

use remex_core::codec::DisconnectReason;

use super::session;
use crate::db::Db;

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetLogs {}
impl Handler<GetLogs> for Db {
  type Result = Vec<String>;
  fn handle(&mut self, _msg: GetLogs, _ctx: &mut Context<Self>) -> Self::Result {
    let _lgs = futures::executor::block_on(async {
      sqlx::query("SELECT * FROM logs")
        .fetch_all(&self.pool)
        .await
        .unwrap()
    });
    vec!["bob".to_owned()]
  }
}

#[derive(Message)]
#[rtype(result = "()")]
struct NewLog {
  client: String,
  message: String,
  time_logged: chrono::NaiveDateTime,
}
impl Handler<NewLog> for Db {
  type Result = ();
  fn handle(&mut self, msg: NewLog, _ctx: &mut Context<Self>) {
    futures::executor::block_on(self.new_log(&msg.client, &msg.message, msg.time_logged));
  }
}
