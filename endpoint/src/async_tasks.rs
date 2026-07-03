pub mod db_heartbeat;
pub mod jobs;

use actix::prelude::*;
use surrealdb::{engine::any::Any, Surreal};

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ConnectionReady {
    pub db: Option<Surreal<Any>>,
    pub client_id: Option<String>,
}
