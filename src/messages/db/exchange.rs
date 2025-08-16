use actix::{Addr, AsyncContext};
use actix::{Context, Handler, Message};
use tracing::{error, warn};

use crate::db;
use crate::endpoint::executor::Executor;
use crate::endpoint::Endpoint;

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
      match db::executors::get_executor_from_machineid(&pool, m.identity.machineid.clone()).await {
        Ok(dbexecutors) => {
          m.session.do_send(super::session::exchange::ExecutorList {
            identity: m.identity.clone(),
            executors: dbexecutors
              .iter()
              .map(|e| Executor {
                id: e.id.clone(),
                name: e.name.clone(),
                command: e.command.clone(),
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
