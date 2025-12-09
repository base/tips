import { type NextRequest, NextResponse } from "next/server";
import {
  type BundleEvent,
  type BundleHistory,
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

export type BundleEventWithId = BundleEvent & { bundleId: string };

export interface TransactionHistoryResponse {
  hash: string;
  bundle_ids: string[];
  history: BundleEventWithId[];
}

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ hash: string }> }
) {
  try {
    const { hash } = await params;

    const metadata = await getTransactionMetadataByHash(hash);

    if (!metadata) {
      return NextResponse.json(
        { error: "Transaction not found" },
        { status: 404 }
      );
    }

    console.log("metadata", metadata);

    // Fetch ALL bundle histories in parallel
    const bundleHistories = await Promise.all(
      metadata.bundle_ids.map((id) => getBundleHistory(id))
    );

    // Filter out nulls and merge all events, tagging each with its bundleId
    const allEvents: BundleEventWithId[] = bundleHistories
      .map((bundle, index) => ({
        bundle,
        bundleId: metadata.bundle_ids[index],
      }))
      .filter(
        (item): item is { bundle: BundleHistory; bundleId: string } =>
          item.bundle !== null
      )
      .flatMap(({ bundle, bundleId }) =>
        bundle.history.map((event) => ({ ...event, bundleId }))
      );

    if (allEvents.length === 0) {
      return NextResponse.json(
        { error: "No bundle history found" },
        { status: 404 }
      );
    }

    // Sort by timestamp
    allEvents.sort((a, b) => (a.data.timestamp ?? 0) - (b.data.timestamp ?? 0));

    // Deduplicate by event key
    const uniqueEvents = allEvents.filter(
      (event, index, self) =>
        index === self.findIndex((e) => e.data?.key === event.data?.key)
    );

    const response: TransactionHistoryResponse = {
      hash,
      bundle_ids: metadata.bundle_ids,
      history: uniqueEvents,
    };

    return NextResponse.json(response);
  } catch (error) {
    console.error("Error fetching transaction data:", error);
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 }
    );
  }
}
