import {
  GetObjectCommand,
  PutObjectCommand,
  S3Client,
  type S3ClientConfig,
} from "@aws-sdk/client-s3";

function createS3Client(): S3Client {
  const configType = process.env.TIPS_UI_S3_CONFIG_TYPE || "aws";
  const region = process.env.TIPS_UI_AWS_REGION || "us-east-1";

  if (configType === "manual") {
    console.log("Using Manual S3 configuration");
    const config: S3ClientConfig = {
      region,
      forcePathStyle: true,
    };

    if (process.env.TIPS_UI_S3_ENDPOINT) {
      config.endpoint = process.env.TIPS_UI_S3_ENDPOINT;
    }

    if (
      process.env.TIPS_UI_S3_ACCESS_KEY_ID &&
      process.env.TIPS_UI_S3_SECRET_ACCESS_KEY
    ) {
      config.credentials = {
        accessKeyId: process.env.TIPS_UI_S3_ACCESS_KEY_ID,
        secretAccessKey: process.env.TIPS_UI_S3_SECRET_ACCESS_KEY,
      };
    }

    return new S3Client(config);
  }

  console.log("Using AWS S3 configuration");
  return new S3Client({
    region,
  });
}

const s3Client = createS3Client();

const BUCKET_NAME = process.env.TIPS_UI_S3_BUCKET_NAME || "tips";

export interface TransactionMetadata {
  bundle_ids: string[];
  sender: string;
  nonce: string;
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
  } catch (_error) {
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

export interface BundleTransaction {
  signer: string;
  type: string;
  chainId: string;
  nonce: string;
  gas: string;
  maxFeePerGas: string;
  maxPriorityFeePerGas: string;
  to: string;
  value: string;
  accessList: unknown[];
  input: string;
  r: string;
  s: string;
  yParity: string;
  v: string;
  hash: string;
}

export interface MeterBundleResult {
  coinbaseDiff: string;
  ethSentToCoinbase: string;
  fromAddress: string;
  gasFees: string;
  gasPrice: string;
  gasUsed: number;
  toAddress: string;
  txHash: string;
  value: string;
  executionTimeUs: number;
}

export interface MeterBundleResponse {
  bundleGasPrice: string;
  bundleHash: string;
  coinbaseDiff: string;
  ethSentToCoinbase: string;
  gasFees: string;
  results: MeterBundleResult[];
  stateBlockNumber: number;
  totalGasUsed: number;
  totalExecutionTimeUs: number;
}

export interface BundleData {
  uuid: string;
  txs: BundleTransaction[];
  block_number: string;
  max_timestamp: number;
  reverting_tx_hashes: string[];
  meter_bundle_response: MeterBundleResponse;
}

export interface BundleEventData {
  key: string;
  timestamp: number;
  bundle?: BundleData;
  block_number?: number;
  block_hash?: string;
  builder?: string;
  flashblock_index?: number;
  reason?: string;
}

export interface BundleEvent {
  event: string;
  data: BundleEventData;
}

export interface BundleHistory {
  history: BundleEvent[];
}

export async function getBundleHistory(
  bundleId: string,
): Promise<BundleHistory | null> {
  const key = `bundles/${bundleId}`;
  const content = await getObjectContent(key);

  if (!content) {
    return null;
  }

  try {
    return JSON.parse(content) as BundleHistory;
  } catch (error) {
    console.error(
      `Failed to parse bundle history for bundle ${bundleId}:`,
      error,
    );
    return null;
  }
}

export interface BlockTransaction {
  hash: string;
  from: string;
  to: string | null;
  gasUsed: bigint;
  executionTimeUs: number | null;
  bundleId: string | null;
  index: number;
}

export interface BlockData {
  hash: string;
  number: bigint;
  timestamp: bigint;
  transactions: BlockTransaction[];
  gasUsed: bigint;
  gasLimit: bigint;
  cachedAt: number;
}

export async function getBlockFromCache(
  blockHash: string,
): Promise<BlockData | null> {
  const key = `blocks/${blockHash}`;
  const content = await getObjectContent(key);

  if (!content) {
    return null;
  }

  try {
    const parsed = JSON.parse(content);
    return {
      ...parsed,
      number: BigInt(parsed.number),
      timestamp: BigInt(parsed.timestamp),
      gasUsed: BigInt(parsed.gasUsed),
      gasLimit: BigInt(parsed.gasLimit),
      transactions: parsed.transactions.map(
        (tx: BlockTransaction & { gasUsed: string }) => ({
          ...tx,
          gasUsed: BigInt(tx.gasUsed),
        }),
      ),
    } as BlockData;
  } catch (error) {
    console.error(`Failed to parse block data for hash ${blockHash}:`, error);
    return null;
  }
}

export async function cacheBlockData(blockData: BlockData): Promise<void> {
  const key = `blocks/${blockData.hash}`;

  try {
    const command = new PutObjectCommand({
      Bucket: BUCKET_NAME,
      Key: key,
      Body: JSON.stringify(blockData, (_, value) =>
        typeof value === "bigint" ? value.toString() : value,
      ),
      ContentType: "application/json",
    });

    await s3Client.send(command);
  } catch (error) {
    console.error(`Failed to cache block data for ${blockData.hash}:`, error);
  }
}
