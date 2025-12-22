import { type NextRequest, NextResponse } from "next/server";
import { type Block, createPublicClient, type Hash, http } from "viem";
import { mainnet } from "viem/chains";
import {
  type BlockData,
  type BlockTransaction,
  cacheBlockData,
  getBlockFromCache,
  getBundleHistory,
  getTransactionMetadataByHash,
  type MeterBundleResult,
} from "@/lib/s3";

function serializeBlockData(block: BlockData) {
  return {
    ...block,
    number: block.number.toString(),
    timestamp: block.timestamp.toString(),
    gasUsed: block.gasUsed.toString(),
    gasLimit: block.gasLimit.toString(),
    transactions: block.transactions.map((tx) => ({
      ...tx,
      gasUsed: tx.gasUsed.toString(),
    })),
  };
}

const RPC_URL = process.env.TIPS_UI_RPC_URL || "http://localhost:8545";

const client = createPublicClient({
  chain: mainnet,
  transport: http(RPC_URL),
});

async function fetchBlockFromRpc(
  blockHash: string,
): Promise<Block<bigint, true> | null> {
  try {
    const block = await client.getBlock({
      blockHash: blockHash as Hash,
      includeTransactions: true,
    });
    return block;
  } catch (error) {
    console.error("Failed to fetch block from RPC:", error);
    return null;
  }
}

// On OP Stack, the first transaction (index 0) is the L1 attributes deposit transaction.
// This is not a perfect check (ideally we'd check tx.type === 'deposit' or type 0x7e),
// but sufficient for filtering out system transactions that don't need simulation data.
function isSystemTransaction(tx: BlockTransaction): boolean {
  return tx.index === 0;
}

async function enrichTransactionWithBundleData(
  txHash: string,
): Promise<{ bundleId: string | null; executionTimeUs: number | null }> {
  const metadata = await getTransactionMetadataByHash(txHash);
  if (!metadata || metadata.bundle_ids.length === 0) {
    return { bundleId: null, executionTimeUs: null };
  }

  const bundleId = metadata.bundle_ids[0];
  const bundleHistory = await getBundleHistory(bundleId);
  if (!bundleHistory) {
    return { bundleId, executionTimeUs: null };
  }

  const receivedEvent = bundleHistory.history.find(
    (e) => e.event === "Received",
  );
  if (!receivedEvent?.data?.bundle?.meter_bundle_response?.results) {
    return { bundleId, executionTimeUs: null };
  }

  const txResult = receivedEvent.data.bundle.meter_bundle_response.results.find(
    (r: MeterBundleResult) => r.txHash.toLowerCase() === txHash.toLowerCase(),
  );

  return {
    bundleId,
    executionTimeUs: txResult?.executionTimeUs ?? null,
  };
}

async function refetchMissingTransactionSimulations(
  block: BlockData,
): Promise<{ updatedBlock: BlockData; hasUpdates: boolean }> {
  const transactionsToRefetch = block.transactions.filter(
    (tx) => tx.bundleId === null && !isSystemTransaction(tx),
  );

  if (transactionsToRefetch.length === 0) {
    return { updatedBlock: block, hasUpdates: false };
  }

  const refetchResults = await Promise.all(
    transactionsToRefetch.map(async (tx) => {
      const { bundleId, executionTimeUs } =
        await enrichTransactionWithBundleData(tx.hash);
      return { hash: tx.hash, bundleId, executionTimeUs };
    }),
  );

  let hasUpdates = false;
  const updatedTransactions = block.transactions.map((tx) => {
    const refetchResult = refetchResults.find((r) => r.hash === tx.hash);
    if (refetchResult && refetchResult.bundleId !== null) {
      hasUpdates = true;
      return {
        ...tx,
        bundleId: refetchResult.bundleId,
        executionTimeUs: refetchResult.executionTimeUs,
      };
    }
    return tx;
  });

  return {
    updatedBlock: {
      ...block,
      transactions: updatedTransactions,
      cachedAt: hasUpdates ? Date.now() : block.cachedAt,
    },
    hasUpdates,
  };
}

export async function GET(
  _request: NextRequest,
  { params }: { params: Promise<{ hash: string }> },
) {
  try {
    const { hash } = await params;

    const cachedBlock = await getBlockFromCache(hash);
    if (cachedBlock) {
      const { updatedBlock, hasUpdates } =
        await refetchMissingTransactionSimulations(cachedBlock);

      if (hasUpdates) {
        await cacheBlockData(updatedBlock);
      }

      return NextResponse.json(serializeBlockData(updatedBlock));
    }

    const rpcBlock = await fetchBlockFromRpc(hash);
    if (!rpcBlock || !rpcBlock.hash || !rpcBlock.number) {
      return NextResponse.json({ error: "Block not found" }, { status: 404 });
    }

    const transactions: BlockTransaction[] = await Promise.all(
      rpcBlock.transactions.map(async (tx, index) => {
        const { bundleId, executionTimeUs } =
          await enrichTransactionWithBundleData(tx.hash);
        return {
          hash: tx.hash,
          from: tx.from,
          to: tx.to,
          gasUsed: tx.gas,
          executionTimeUs,
          bundleId,
          index,
        };
      }),
    );

    const blockData: BlockData = {
      hash: rpcBlock.hash,
      number: rpcBlock.number,
      timestamp: rpcBlock.timestamp,
      transactions,
      gasUsed: rpcBlock.gasUsed,
      gasLimit: rpcBlock.gasLimit,
      cachedAt: Date.now(),
    };

    await cacheBlockData(blockData);

    return NextResponse.json(serializeBlockData(blockData));
  } catch (error) {
    console.error("Error fetching block data:", error);
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 },
    );
  }
}
