import { type NextRequest, NextResponse } from "next/server";
import {
  type BundleEvent,
  getBundleHistory,
  getTransactionMetadataByHash,
} from "@/lib/s3";

export interface TransactionEvent {
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

export interface TransactionHistoryResponse {
  hash: string;
  bundle_ids: string[];
  history: BundleEvent[];
}

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ hash: string }> },
) {
  try {
    const { hash } = await params;

    const metadata = await getTransactionMetadataByHash(hash);

    if (!metadata) {
      return NextResponse.json(
        { error: "Transaction not found" },
        { status: 404 },
      );
    }

    // TODO: Can be in multiple bundles
    const bundle = await getBundleHistory(metadata.bundle_ids[0]);
    if (!bundle) {
      return NextResponse.json({ error: "Bundle not found" }, { status: 404 });
    }

    const response: TransactionHistoryResponse = {
      hash,
      bundle_ids: metadata.bundle_ids,
      history: bundle.history,
    };

    return NextResponse.json(response);
  } catch (error) {
    console.error("Error fetching transaction data:", error);
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 },
    );
  }
}
