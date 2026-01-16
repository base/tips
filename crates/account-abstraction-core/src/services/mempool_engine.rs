use super::interfaces::event_source::EventSource;
use crate::domain::{events::MempoolEvent, mempool::Mempool};
use std::sync::Arc;
use tips_audit_lib::{UserOpDropReason, UserOpEvent};
use tokio::sync::{RwLock, mpsc};
use tracing::{info, warn};

pub struct MempoolEngine<T: Mempool> {
    mempool: Arc<RwLock<T>>,
    event_source: Arc<dyn EventSource>,
    audit_sender: Option<mpsc::UnboundedSender<UserOpEvent>>,
}

impl<T: Mempool> MempoolEngine<T> {
    pub fn new(mempool: Arc<RwLock<T>>, event_source: Arc<dyn EventSource>) -> MempoolEngine<T> {
        Self {
            mempool,
            event_source,
            audit_sender: None,
        }
    }

    pub fn with_audit_sender(
        mempool: Arc<RwLock<T>>,
        event_source: Arc<dyn EventSource>,
        audit_sender: mpsc::UnboundedSender<UserOpEvent>,
    ) -> MempoolEngine<T> {
        Self {
            mempool,
            event_source,
            audit_sender: Some(audit_sender),
        }
    }

    pub fn get_mempool(&self) -> Arc<RwLock<T>> {
        Arc::clone(&self.mempool)
    }

    pub async fn run(&self) {
        loop {
            if let Err(err) = self.process_next().await {
                warn!(error = %err, "Mempool engine error, continuing");
            }
        }
    }

    pub async fn process_next(&self) -> anyhow::Result<()> {
        let event = self.event_source.receive().await?;
        self.handle_event(event).await
    }

    async fn handle_event(&self, event: MempoolEvent) -> anyhow::Result<()> {
        info!(
            event = ?event,
            "Mempool engine handling event"
        );
        match event {
            MempoolEvent::UserOpAdded {
                user_op,
                entry_point,
            } => {
                self.mempool.write().await.add_operation(&user_op)?;
                self.emit_audit_event(UserOpEvent::AddedToMempool {
                    user_op_hash: user_op.hash,
                    sender: user_op.operation.sender(),
                    entry_point,
                    nonce: user_op.operation.nonce(),
                });
            }
            MempoolEvent::UserOpIncluded {
                user_op,
                block_number,
                tx_hash,
            } => {
                self.mempool.write().await.remove_operation(&user_op.hash)?;
                self.emit_audit_event(UserOpEvent::Included {
                    user_op_hash: user_op.hash,
                    block_number,
                    tx_hash,
                });
            }
            MempoolEvent::UserOpDropped { user_op, reason } => {
                self.mempool.write().await.remove_operation(&user_op.hash)?;
                self.emit_audit_event(UserOpEvent::Dropped {
                    user_op_hash: user_op.hash,
                    reason: UserOpDropReason::Invalid(reason),
                });
            }
        }
        Ok(())
    }

    fn emit_audit_event(&self, event: UserOpEvent) {
        if let Some(sender) = &self.audit_sender
            && let Err(e) = sender.send(event)
        {
            warn!(error = %e, "Failed to send audit event");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        mempool::PoolConfig,
        types::{VersionedUserOperation, WrappedUserOperation},
    };
    use crate::infrastructure::in_memory::mempool::InMemoryMempool;
    use crate::services::interfaces::event_source::EventSource;
    use alloy_primitives::{Address, FixedBytes, Uint};
    use alloy_rpc_types::erc4337;
    use async_trait::async_trait;
    use tokio::sync::Mutex;

    fn make_wrapped_op(max_fee: u128, hash: [u8; 32]) -> WrappedUserOperation {
        let op = VersionedUserOperation::UserOperation(erc4337::UserOperation {
            sender: Address::ZERO,
            nonce: Uint::from(0u64),
            init_code: Default::default(),
            call_data: Default::default(),
            call_gas_limit: Uint::from(100_000u64),
            verification_gas_limit: Uint::from(100_000u64),
            pre_verification_gas: Uint::from(21_000u64),
            max_fee_per_gas: Uint::from(max_fee),
            max_priority_fee_per_gas: Uint::from(max_fee),
            paymaster_and_data: Default::default(),
            signature: Default::default(),
        });

        WrappedUserOperation {
            operation: op,
            hash: FixedBytes::from(hash),
        }
    }

    struct MockEventSource {
        events: Mutex<Vec<MempoolEvent>>,
    }

    impl MockEventSource {
        fn new(events: Vec<MempoolEvent>) -> Self {
            Self {
                events: Mutex::new(events),
            }
        }
    }

    #[async_trait]
    impl EventSource for MockEventSource {
        async fn receive(&self) -> anyhow::Result<MempoolEvent> {
            let mut guard = self.events.lock().await;
            if guard.is_empty() {
                Err(anyhow::anyhow!("no more events"))
            } else {
                Ok(guard.remove(0))
            }
        }
    }

    #[tokio::test]
    async fn handle_add_operation() {
        let mempool = Arc::new(RwLock::new(InMemoryMempool::new(PoolConfig::default())));

        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);

        let add_event = MempoolEvent::UserOpAdded {
            user_op: wrapped.clone(),
            entry_point: Address::ZERO,
        };
        let mock_source = Arc::new(MockEventSource::new(vec![add_event]));

        let engine = MempoolEngine::new(mempool.clone(), mock_source);

        engine.process_next().await.unwrap();
        let items: Vec<_> = mempool.read().await.get_top_operations(10).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].hash, FixedBytes::from(op_hash));
    }

    #[tokio::test]
    async fn remove_operation_should_remove_from_mempool() {
        let mempool = Arc::new(RwLock::new(InMemoryMempool::new(PoolConfig::default())));
        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);
        let add_event = MempoolEvent::UserOpAdded {
            user_op: wrapped.clone(),
            entry_point: Address::ZERO,
        };
        let remove_event = MempoolEvent::UserOpDropped {
            user_op: wrapped.clone(),
            reason: "test".to_string(),
        };
        let mock_source = Arc::new(MockEventSource::new(vec![add_event, remove_event]));

        let engine = MempoolEngine::new(mempool.clone(), mock_source);
        engine.process_next().await.unwrap();
        let items: Vec<_> = mempool.read().await.get_top_operations(10).collect();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].hash, FixedBytes::from(op_hash));
        engine.process_next().await.unwrap();
        let items: Vec<_> = mempool.read().await.get_top_operations(10).collect();
        assert_eq!(items.len(), 0);
    }

    #[tokio::test]
    async fn audit_event_emitted_on_user_op_added() {
        let mempool = Arc::new(RwLock::new(InMemoryMempool::new(PoolConfig::default())));
        let (audit_tx, mut audit_rx) = mpsc::unbounded_channel::<UserOpEvent>();

        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);
        let entry_point = Address::with_last_byte(42);

        let add_event = MempoolEvent::UserOpAdded {
            user_op: wrapped.clone(),
            entry_point,
        };
        let mock_source = Arc::new(MockEventSource::new(vec![add_event]));

        let engine = MempoolEngine::with_audit_sender(mempool.clone(), mock_source, audit_tx);
        engine.process_next().await.unwrap();

        let audit_event = audit_rx.try_recv().expect("Should receive audit event");
        match audit_event {
            UserOpEvent::AddedToMempool {
                user_op_hash,
                sender,
                entry_point: ep,
                nonce,
            } => {
                assert_eq!(user_op_hash, FixedBytes::from(op_hash));
                assert_eq!(sender, Address::ZERO);
                assert_eq!(ep, entry_point);
                assert_eq!(nonce, Uint::from(0u64));
            }
            _ => panic!("Expected AddedToMempool event"),
        }
    }

    #[tokio::test]
    async fn audit_event_emitted_on_user_op_dropped() {
        let mempool = Arc::new(RwLock::new(InMemoryMempool::new(PoolConfig::default())));
        let (audit_tx, mut audit_rx) = mpsc::unbounded_channel::<UserOpEvent>();

        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);

        let add_event = MempoolEvent::UserOpAdded {
            user_op: wrapped.clone(),
            entry_point: Address::ZERO,
        };
        let drop_event = MempoolEvent::UserOpDropped {
            user_op: wrapped.clone(),
            reason: "gas too low".to_string(),
        };
        let mock_source = Arc::new(MockEventSource::new(vec![add_event, drop_event]));

        let engine = MempoolEngine::with_audit_sender(mempool.clone(), mock_source, audit_tx);
        engine.process_next().await.unwrap();
        engine.process_next().await.unwrap();

        let _ = audit_rx
            .try_recv()
            .expect("Should receive AddedToMempool event");
        let audit_event = audit_rx.try_recv().expect("Should receive Dropped event");
        match audit_event {
            UserOpEvent::Dropped {
                user_op_hash,
                reason,
            } => {
                assert_eq!(user_op_hash, FixedBytes::from(op_hash));
                match reason {
                    UserOpDropReason::Invalid(msg) => assert_eq!(msg, "gas too low"),
                    _ => panic!("Expected Invalid reason"),
                }
            }
            _ => panic!("Expected Dropped event"),
        }
    }

    #[tokio::test]
    async fn audit_event_emitted_on_user_op_included() {
        let mempool = Arc::new(RwLock::new(InMemoryMempool::new(PoolConfig::default())));
        let (audit_tx, mut audit_rx) = mpsc::unbounded_channel::<UserOpEvent>();

        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);
        let tx_hash = FixedBytes::from([2u8; 32]);

        let add_event = MempoolEvent::UserOpAdded {
            user_op: wrapped.clone(),
            entry_point: Address::ZERO,
        };
        let include_event = MempoolEvent::UserOpIncluded {
            user_op: wrapped.clone(),
            block_number: 12345,
            tx_hash,
        };
        let mock_source = Arc::new(MockEventSource::new(vec![add_event, include_event]));

        let engine = MempoolEngine::with_audit_sender(mempool.clone(), mock_source, audit_tx);
        engine.process_next().await.unwrap();
        engine.process_next().await.unwrap();

        let _ = audit_rx
            .try_recv()
            .expect("Should receive AddedToMempool event");
        let audit_event = audit_rx.try_recv().expect("Should receive Included event");
        match audit_event {
            UserOpEvent::Included {
                user_op_hash,
                block_number,
                tx_hash: th,
            } => {
                assert_eq!(user_op_hash, FixedBytes::from(op_hash));
                assert_eq!(block_number, 12345);
                assert_eq!(th, tx_hash);
            }
            _ => panic!("Expected Included event"),
        }
    }

    #[tokio::test]
    async fn no_audit_event_when_sender_is_none() {
        let mempool = Arc::new(RwLock::new(InMemoryMempool::new(PoolConfig::default())));

        let op_hash = [1u8; 32];
        let wrapped = make_wrapped_op(1_000, op_hash);

        let add_event = MempoolEvent::UserOpAdded {
            user_op: wrapped.clone(),
            entry_point: Address::ZERO,
        };
        let mock_source = Arc::new(MockEventSource::new(vec![add_event]));

        let engine = MempoolEngine::new(mempool.clone(), mock_source);
        engine.process_next().await.unwrap();

        let items: Vec<_> = mempool.read().await.get_top_operations(10).collect();
        assert_eq!(items.len(), 1);
    }
}
