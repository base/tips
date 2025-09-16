use crate::types::{BundleId, MempoolEvent, TransactionId};
use anyhow::Result;
use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use tracing::info;

#[derive(Debug)]
pub enum S3Key {
    Bundle(BundleId),
    TransactionByHash(String),
    CanonicalTransaction { sender: String, nonce: String },
}

impl fmt::Display for S3Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            S3Key::Bundle(bundle_id) => write!(f, "bundles/{}", bundle_id),
            S3Key::TransactionByHash(hash) => write!(f, "transactions/by_hash/{}", hash),
            S3Key::CanonicalTransaction { sender, nonce } => {
                write!(f, "transactions/canonical/{}/{}", sender, nonce)
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionMetadata {
    pub bundle_ids: Vec<BundleId>,
    pub sender: String,
    pub nonce: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CanonicalTransactionEvent {
    pub event_log: Vec<MempoolEvent>,
}

#[async_trait]
pub trait MempoolEventWriter {
    async fn write_event(&self, event: MempoolEvent) -> Result<()>;
}

pub struct S3MempoolEventWriter {
    s3_client: S3Client,
    bucket: String,
}

impl S3MempoolEventWriter {
    pub fn new(s3_client: S3Client, bucket: String) -> Self {
        Self { s3_client, bucket }
    }

    async fn archive_event(&self, event: MempoolEvent) -> Result<()> {
        let bundle_id = event.bundle_id();
        let transaction_ids = event.transaction_ids();

        self.update_bundle_index(bundle_id, &transaction_ids)
            .await?;

        for tx_id in &transaction_ids {
            self.update_transaction_by_hash_index(tx_id, bundle_id)
                .await?;
            self.update_canonical_transaction_log(tx_id, &event).await?;
        }

        Ok(())
    }

    async fn update_bundle_index(
        &self,
        bundle_id: BundleId,
        transaction_ids: &[TransactionId],
    ) -> Result<()> {
        let s3_key = S3Key::Bundle(bundle_id);
        let key = s3_key.to_string();

        let existing_hashes = self.get_bundle_transaction_hashes(bundle_id).await?;
        info!(
            bundle_id = %bundle_id,
            existing_hash_count = existing_hashes.len(),
            existing_hashes = ?existing_hashes,
            "Retrieved existing bundle hashes"
        );

        let mut all_hashes: HashSet<String> = existing_hashes.into_iter().collect();
        for tx_id in transaction_ids {
            let hash_str = format!("{:?}", tx_id.hash);
            info!(
                bundle_id = %bundle_id,
                tx_hash = %hash_str,
                "Adding transaction hash to bundle index"
            );
            all_hashes.insert(hash_str);
        }

        let hashes_vec: Vec<String> = all_hashes.into_iter().collect();
        let content = serde_json::to_string(&hashes_vec)?;
        info!(
            bundle_id = %bundle_id,
            final_hash_count = hashes_vec.len(),
            final_hashes = ?hashes_vec,
            "Final bundle index content"
        );

        self.put_object_idempotent(&key, content.into_bytes())
            .await?;

        info!(
            bundle_id = %bundle_id,
            transaction_count = hashes_vec.len(),
            s3_key = %key,
            "Updated bundle index"
        );
        Ok(())
    }

    async fn update_transaction_by_hash_index(
        &self,
        tx_id: &TransactionId,
        bundle_id: BundleId,
    ) -> Result<()> {
        let s3_key = S3Key::TransactionByHash(format!("{:?}", tx_id.hash));
        let key = s3_key.to_string();

        let existing_metadata = self
            .get_transaction_metadata_by_hash(&tx_id.hash.to_string())
            .await?;

        let mut bundle_ids = existing_metadata.map(|m| m.bundle_ids).unwrap_or_default();
        if !bundle_ids.contains(&bundle_id) {
            bundle_ids.push(bundle_id);
        }

        let metadata = TransactionMetadata {
            bundle_ids,
            sender: format!("{:?}", tx_id.sender),
            nonce: tx_id.nonce.to_string(),
        };

        let content = serde_json::to_string(&metadata)?;
        self.put_object_idempotent(&key, content.into_bytes())
            .await?;

        info!(
            tx_hash = ?tx_id.hash,
            bundle_id = %bundle_id,
            s3_key = %key,
            "Updated transaction by hash index"
        );
        Ok(())
    }

    async fn update_canonical_transaction_log(
        &self,
        tx_id: &TransactionId,
        event: &MempoolEvent,
    ) -> Result<()> {
        let s3_key = S3Key::CanonicalTransaction {
            sender: format!("{:?}", tx_id.sender),
            nonce: tx_id.nonce.to_string(),
        };
        let key = s3_key.to_string();

        let existing_log = self.get_canonical_transaction_log(tx_id).await?;

        let mut event_log = existing_log.map(|l| l.event_log).unwrap_or_default();
        event_log.push(event.clone());

        let canonical_event = CanonicalTransactionEvent { event_log };
        let content = serde_json::to_string(&canonical_event)?;

        self.put_object_idempotent(&key, content.into_bytes())
            .await?;

        info!(
            tx_sender = ?tx_id.sender,
            tx_nonce = %tx_id.nonce,
            s3_key = %key,
            event_count = canonical_event.event_log.len(),
            "Updated canonical transaction log"
        );
        Ok(())
    }

    async fn put_object_idempotent(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let md5_digest = md5::compute(&data);
        let content_hash_hex = format!("{:x}", md5_digest);

        if let Ok(existing) = self.get_object_etag(key).await {
            if existing.trim_matches('"') == content_hash_hex {
                info!(
                    s3_key = %key,
                    content_hash = %content_hash_hex,
                    "Object already exists with same content, skipping"
                );
                return Ok(());
            }
        }

        let data_size = data.len();
        let body = ByteStream::from(data);

        self.s3_client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await?;

        info!(
            s3_key = %key,
            content_hash = %content_hash_hex,
            data_size = data_size,
            "Successfully uploaded object to S3"
        );
        Ok(())
    }

    async fn get_object_etag(&self, key: &str) -> Result<String> {
        let response = self
            .s3_client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        Ok(response.e_tag().unwrap_or("").to_string())
    }

    async fn get_bundle_transaction_hashes(&self, bundle_id: BundleId) -> Result<Vec<String>> {
        let s3_key = S3Key::Bundle(bundle_id);
        let key = s3_key.to_string();

        match self.get_object_content(&key).await {
            Ok(content) => Ok(serde_json::from_str(&content)?),
            Err(_) => Ok(Vec::new()),
        }
    }

    async fn get_transaction_metadata_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<TransactionMetadata>> {
        let s3_key = S3Key::TransactionByHash(hash.to_string());
        let key = s3_key.to_string();

        match self.get_object_content(&key).await {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(_) => Ok(None),
        }
    }

    async fn get_canonical_transaction_log(
        &self,
        tx_id: &TransactionId,
    ) -> Result<Option<CanonicalTransactionEvent>> {
        let s3_key = S3Key::CanonicalTransaction {
            sender: format!("{:?}", tx_id.sender),
            nonce: tx_id.nonce.to_string(),
        };
        let key = s3_key.to_string();

        match self.get_object_content(&key).await {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(_) => Ok(None),
        }
    }

    async fn get_object_content(&self, key: &str) -> Result<String> {
        let response = self
            .s3_client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;

        let body = response.body.collect().await?;
        Ok(String::from_utf8(body.into_bytes().to_vec())?)
    }
}

#[async_trait]
impl MempoolEventWriter for S3MempoolEventWriter {
    async fn write_event(&self, event: MempoolEvent) -> Result<()> {
        self.archive_event(event).await
    }
}
