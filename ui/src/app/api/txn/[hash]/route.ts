import { type NextRequest, NextResponse } from "next/server";
import {
  getCanonicalTransactionLog,
  getTransactionMetadataByHash,
} from "../../../../lib/s3";

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
  events: TransactionEvent[];
  metadata?: {
    bundle_ids: string[];
    sender: string;
    nonce: string;
  };
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

    const canonicalLog = await getCanonicalTransactionLog(
      metadata.sender,
      metadata.nonce,
    );

    const response: TransactionHistoryResponse = {
      hash,
      events: canonicalLog?.event_log || [],
      metadata,
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
