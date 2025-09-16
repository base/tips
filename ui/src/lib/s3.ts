import { GetObjectCommand, S3Client } from "@aws-sdk/client-s3";

const s3Client = new S3Client({
  region: process.env.AWS_REGION || "us-east-1",
});

const BUCKET_NAME = process.env.S3_BUCKET_NAME || "tips-audit-data";

export interface TransactionMetadata {
  bundle_ids: string[];
  sender: string;
  nonce: string;
}

export interface MempoolEvent {
  type: string;
  data: {
    bundle_id?: string;
    transactions?: Array<{
      id: {
        sender: string;
        nonce: string;
        hash: string;
      };
      data: string;
    }>;
    transaction_ids?: Array<{
      sender: string;
      nonce: string;
      hash: string;
    }>;
    block_number?: number;
    flashblock_index?: number;
    block_hash?: string;
  };
}

export interface CanonicalTransactionEvent {
  event_log: MempoolEvent[];
}

async function getObjectContent(key: string): Promise<string | null> {
  try {
    const command = new GetObjectCommand({
      Bucket: BUCKET_NAME,
      Key: key,
    });

    const response = await s3Client.send(command);
    const body = await response.Body?.transformToString();
    return body || null;
  } catch (error) {
    console.error(`Failed to get S3 object ${key}:`, error);
    return null;
  }
}

export async function getTransactionMetadataByHash(
  hash: string,
): Promise<TransactionMetadata | null> {
  const key = `transactions/by_hash/${hash}`;
  const content = await getObjectContent(key);

  if (!content) {
    return null;
  }

  try {
    return JSON.parse(content) as TransactionMetadata;
  } catch (error) {
    console.error(
      `Failed to parse transaction metadata for hash ${hash}:`,
      error,
    );
    return null;
  }
}

export async function getCanonicalTransactionLog(
  sender: string,
  nonce: string,
): Promise<CanonicalTransactionEvent | null> {
  const key = `transactions/canonical/${sender}/${nonce}`;
  const content = await getObjectContent(key);

  if (!content) {
    return null;
  }

  try {
    return JSON.parse(content) as CanonicalTransactionEvent;
  } catch (error) {
    console.error(
      `Failed to parse canonical transaction log for ${sender}/${nonce}:`,
      error,
    );
    return null;
  }
}

export async function getBundleTransactionHashes(
  bundleId: string,
): Promise<string[] | null> {
  const key = `bundles/${bundleId}`;
  const content = await getObjectContent(key);

  if (!content) {
    return null;
  }

  try {
    return JSON.parse(content) as string[];
  } catch (error) {
    console.error(
      `Failed to parse bundle transaction hashes for bundle ${bundleId}:`,
      error,
    );
    return null;
  }
}
