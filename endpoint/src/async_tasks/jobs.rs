use std::sync::Arc;

use remex_core::codec;
use tokio::sync::Mutex;

// pub async fn jobs_exec(
//   ctx: Arc<Mutex<crate::Context>>,
//   tx: tokio::sync::mpsc::Sender<codec::ClientRequest>,
// ) {
//   tracing::info!("Starting jobs executor");
//   let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
//   loop {
//     interval.tick().await;
//
//     let jobs_to_exec: Vec<String> = {
//       let ctx_lock = ctx.lock().await;
//       if ctx_lock.authenticated {
//         ctx_lock
//           .cache
//           .jobs
//           .iter()
//           .map(|j| j.job_name.clone())
//           .collect()
//       } else {
//         vec![]
//       }
//     };
//
//     for job_name in jobs_to_exec {
//       // TODO: Implement job execution when database is re-added
//       tracing::info!("Job {} is in TODO state", job_name);
//
//       if tx
//         .send(codec::ClientRequest::JobsRequest(codec::JobsRequest::SendExecutions(
//           job_name,
//           (),
//           (),
//         )))
//         .await
//         .is_err()
//       {
//         return;
//       }
//     }
//   }
// }

