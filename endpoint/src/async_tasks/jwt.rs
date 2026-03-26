use std::sync::Arc;
use tokio::sync::Mutex;

const JWT_REFRESH_INTERVAL_SECS: u64 = 24 * 60 * 60;

pub async fn jwt_refresh(
  ctx: Arc<Mutex<crate::Context>>,
  tx: tokio::sync::mpsc::Sender<remex_core::codec::ClientRequest>,
  mut rx: tokio::sync::mpsc::Receiver<remex_core::codec::ServerResponse>,
) {
  let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(JWT_REFRESH_INTERVAL_SECS));
  
  let mut last_refresh: Option<std::time::Instant> = None;
  
  loop {
    tokio::select! {
      _ = interval.tick() => {
        let should_refresh = {
          let ctx_lock = ctx.lock().await;
          if ctx_lock.authenticated {
            if let Some(last) = last_refresh {
              if last.elapsed().as_secs() >= JWT_REFRESH_INTERVAL_SECS {
                true
              } else {
                false
              }
            } else {
              true
            }
          } else {
            false
          }
        };
        
        if should_refresh {
          tracing::info!("Requesting JWT token refresh");
          if tx.send(remex_core::codec::ClientRequest::RefreshJwt).await.is_err() {
            tracing::error!("Failed to send JWT refresh request, channel closed");
            break;
          }
        }
      }
      msg = rx.recv() => {
        if let Some(remex_core::codec::ServerResponse::JwtRefreshed(token)) = msg {
          let mut ctx_lock = ctx.lock().await;
          ctx_lock.jwt_token = Some(token.clone());
          last_refresh = Some(std::time::Instant::now());
          tracing::info!("JWT token refreshed successfully");
          
          if let Some(_db) = &ctx_lock.db {
            tracing::debug!("DB connection maintained with new JWT");
          }
        }
      }
    }
  }
}
