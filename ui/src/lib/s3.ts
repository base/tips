import {
  GetObjectCommand,
  ListObjectsV2Command,
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

export interface BundleEvent {
  event: string;
  data: {
    key: string;
    timestamp: number;
    bundle?: {
      revertingTxHashes: Array<string>;
    };
  };
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

export async function listAllBundleKeys(): Promise<string[]> {
  try {
    const command = new ListObjectsV2Command({
      Bucket: BUCKET_NAME,
      Prefix: "bundles/",
    });

    const response = await s3Client.send(command);
    const bundleKeys: string[] = [];

    if (response.Contents) {
      for (const object of response.Contents) {
        if (object.Key?.startsWith("bundles/")) {
          const bundleId = object.Key.replace("bundles/", "");
          if (bundleId) {
            bundleKeys.push(bundleId);
          }
        }
      }
    }

    return bundleKeys;
  } catch (error) {
    console.error("Failed to list S3 bundle keys:", error);
    return [];
  }
}
