use std::time::Duration;

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_credential_types::{Credentials, provider::SharedCredentialsProvider};
use aws_sdk_s3::{
    Client as S3Client,
    config::{Builder as S3ConfigBuilder, Region},
};
use tips_audit::storage::{BundleEventS3Reader, BundleHistoryEvent, S3EventReaderWriter};
use tokio::time::{Instant, sleep};
use uuid::Uuid;

const DEFAULT_S3_ENDPOINT: &str = "http://localhost:7000";
const DEFAULT_S3_REGION: &str = "us-east-1";
const DEFAULT_S3_BUCKET: &str = "tips";
const DEFAULT_S3_ACCESS_KEY: &str = "minioadmin";
const DEFAULT_S3_SECRET_KEY: &str = "minioadmin";
const S3_WAIT_TIMEOUT: Duration = Duration::from_secs(30);

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

async fn build_s3_reader() -> Result<S3EventReaderWriter> {
    let endpoint = env_or("TIPS_AUDIT_S3_ENDPOINT", DEFAULT_S3_ENDPOINT);
    let region = env_or("TIPS_AUDIT_S3_REGION", DEFAULT_S3_REGION);
    let bucket = env_or("TIPS_AUDIT_S3_BUCKET", DEFAULT_S3_BUCKET);
    let access_key = env_or("TIPS_AUDIT_S3_ACCESS_KEY_ID", DEFAULT_S3_ACCESS_KEY);
    let secret_key = env_or("TIPS_AUDIT_S3_SECRET_ACCESS_KEY", DEFAULT_S3_SECRET_KEY);

    let creds = Credentials::new(access_key, secret_key, None, None, "tips-system-tests");
    let shared_creds = SharedCredentialsProvider::new(creds);

    let base_config = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region))
        .endpoint_url(endpoint)
        .credentials_provider(shared_creds)
        .load()
        .await;

    let s3_config = S3ConfigBuilder::from(&base_config)
        .force_path_style(true)
        .build();

    let client = S3Client::from_conf(s3_config);
    Ok(S3EventReaderWriter::new(client, bucket))
}

pub async fn wait_for_bundle_history_event(
    bundle_id: Uuid,
    mut predicate: impl FnMut(&BundleHistoryEvent) -> bool,
) -> Result<BundleHistoryEvent> {
    let reader = build_s3_reader()
        .await
        .context("Failed to create S3 reader")?;
    let deadline = Instant::now() + S3_WAIT_TIMEOUT;

    loop {
        if Instant::now() >= deadline {
            anyhow::bail!(
                "Timed out waiting for S3 bundle history for bundle {}",
                bundle_id
            );
        }

        if let Some(history) = reader
            .get_bundle_history(bundle_id)
            .await
            .context("Failed to fetch bundle history from S3")?
        {
            if let Some(event) = history.history.iter().find(|ev| predicate(ev)) {
                return Ok(event.clone());
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}
