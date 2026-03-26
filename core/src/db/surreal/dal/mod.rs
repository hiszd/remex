use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;

pub mod client;
pub mod execution;
pub mod group;
pub mod job;
pub mod log;

pub use client::ClientDal;
pub use execution::ExecutionDal;
pub use group::GroupDal;
pub use job::JobDal;
pub use log::LogDal;
