use std::sync::Arc;
use tokio::sync::Mutex;

const SERVER_OFFLINE_WARNING_SECS: u64 = 5 * 60;
const SERVER_OFFLINE_CRITICAL_SECS: u64 = 15 * 60;

pub async fn server_monitor(
  ctx: Arc<Mutex<crate::Context>>,
) {
  let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
  
  let mut last_connected: Option<std::time::Instant> = None;
  let mut consecutive_failures: u32 = 0;
  
  loop {
    interval.tick().await;
    
    let is_authenticated = {
      let ctx_lock = ctx.lock().await;
      ctx_lock.authenticated
    };
    
    if is_authenticated {
      last_connected = Some(std::time::Instant::now());
      consecutive_failures = 0;
    } else {
      consecutive_failures += 1;
      
      if let Some(last) = last_connected {
        let elapsed = last.elapsed().as_secs();
        
        if elapsed >= SERVER_OFFLINE_CRITICAL_SECS {
          tracing::error!("Server connection lost for {} seconds - CRITICAL", elapsed);
        } else if elapsed >= SERVER_OFFLINE_WARNING_SECS {
          tracing::warn!("Server connection lost for {} seconds", elapsed);
        }
      }
      
      if consecutive_failures > 10 {
        tracing::debug!("Server connection attempts: {}", consecutive_failures);
      }
    }
  }
}
