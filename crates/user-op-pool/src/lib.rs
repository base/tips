pub mod pool;
pub mod source;

use source::UserOpSource;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tracing::error;

pub use pool::{InMemoryUserOpPool, UserOpId, UserOpPoolItem, UserOpStore};
pub use source::KafkaUserOpSource;

/// Connect UserOp sources to the pool (mirrors bundle-pool pattern)
pub fn connect_sources_to_pool<S, P>(
    sources: Vec<S>,
    user_op_rx: mpsc::UnboundedReceiver<UserOpPoolItem>,
    pool: Arc<Mutex<P>>,
) where
    S: UserOpSource + Send + 'static,
    P: UserOpStore + Send + 'static,
{
    // Spawn each source
    for source in sources {
        tokio::spawn(async move {
            if let Err(e) = source.run().await {
                error!(error = %e, "UserOp source failed");
            }
        });
    }

    // Connect receiver to pool
    tokio::spawn(async move {
        let mut user_op_rx = user_op_rx;
        while let Some(user_op) = user_op_rx.recv().await {
            pool.lock().unwrap().add_user_op(user_op);
        }
    });
}

