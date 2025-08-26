use actix::{
  Addr,
  AsyncContext,
  Context,
  Handler,
  Message,
};
use remex_core::{
  endpoint::Endpoint,
  executor::Executor,
};
use tracing::error;

use crate::db;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct SendConfiguration {
  pub identity: Endpoint,
  pub session: Addr<crate::session::RemexSession>,
}

impl Handler<SendConfiguration> for db::Db {
  type Result = ();
  fn handle(&mut self, msg: SendConfiguration, ctx: &mut Context<Self>) -> Self::Result {
    let m = msg.clone();
    let pool = self.pool.clone();
    let fut = async move {
      match db::query::executor::get_executor_from_machineid(&pool, m.identity.machineid.clone())
        .await
      {
        Ok(dbexecutors) => {
          m.session.do_send(super::session::exchange::ExecutorList {
            identity: m.identity.clone(),
            executors: dbexecutors
              .iter()
              .map(|e| Executor {
                id: e.id.clone(),
                name: e.name.clone(),
                command: e.command.clone(),
                status: e.status.clone().into(),
                active: e.active,
                created_at: e.created_at.clone(),
                updated_at: e.updated_at.clone(),
              })
              .collect(),
          });
        }
        Err(e) => {
          error!("Error getting executor: {}", e);
        }
      }
    };
    let fut = actix::fut::wrap_future::<_, Self>(fut);
    ctx.spawn(fut);
  }
}
