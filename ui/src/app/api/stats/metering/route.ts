import { type NextRequest, NextResponse } from "next/server";
import { type BlockData, getBlockFromCache } from "@/lib/s3";

const RPC_URL = process.env.TIPS_UI_RPC_URL || "http://localhost:8545";

export interface MeteringStatsResponse {
  timeWindowStart: number;
  timeWindowEnd: number;
  blockCount: number;
  transactionCount: number;
  stats: {
    avgExecutionTimeUs: number;
    minExecutionTime: {
      timeUs: number;
      txHash: string;
      blockNumber: number;
    } | null;
    maxExecutionTime: {
      timeUs: number;
      txHash: string;
      blockNumber: number;
    } | null;
    p50ExecutionTimeUs: number;
    p95ExecutionTimeUs: number;
    p99ExecutionTimeUs: number;
    avgGasEfficiency: number;
  };
}

async function fetchLatestBlockNumber(): Promise<number | null> {
  try {
    const response = await fetch(RPC_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "eth_blockNumber",
        params: [],
        id: 1,
      }),
    });

    const data = await response.json();
    if (data.error || !data.result) {
      return null;
    }

    return parseInt(data.result, 16);
  } catch (error) {
    console.error("Failed to fetch latest block number:", error);
    return null;
  }
}

async function fetchBlockHashByNumber(
  blockNumber: number,
): Promise<string | null> {
  try {
    const response = await fetch(RPC_URL, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        method: "eth_getBlockByNumber",
        params: [`0x${blockNumber.toString(16)}`, false],
        id: 1,
      }),
    });

    const data = await response.json();
    if (data.error || !data.result) {
      return null;
    }

    return data.result.hash;
  } catch (error) {
    console.error(`Failed to fetch block hash for ${blockNumber}:`, error);
    return null;
  }
}

function calculatePercentile(
  sortedValues: number[],
  percentile: number,
): number {
  if (sortedValues.length === 0) return 0;
  const index = Math.floor(sortedValues.length * percentile);
  return sortedValues[Math.min(index, sortedValues.length - 1)];
}

export async function GET(request: NextRequest) {
  try {
    const searchParams = request.nextUrl.searchParams;
    const blockCount = parseInt(searchParams.get("blocks") || "150", 10);

    const latestBlockNumber = await fetchLatestBlockNumber();
    if (latestBlockNumber === null) {
      return NextResponse.json(
        { error: "Failed to fetch latest block number" },
        { status: 500 },
      );
    }

    const blockNumbers = Array.from(
      { length: blockCount },
      (_, i) => latestBlockNumber - i,
    ).filter((n) => n >= 0);

    const blockHashes = await Promise.all(
      blockNumbers.map(async (num) => ({
        number: num,
        hash: await fetchBlockHashByNumber(num),
      })),
    );

    const blocks = await Promise.all(
      blockHashes
        .filter((b): b is { number: number; hash: string } => b.hash !== null)
        .map(async (b) => ({
          number: b.number,
          data: await getBlockFromCache(b.hash),
        })),
    );

    const validBlocks = blocks.filter((b) => b.data !== null) as Array<{
      number: number;
      data: BlockData;
    }>;

    interface TransactionWithMetering {
      executionTimeUs: number;
      gasUsed: bigint;
      txHash: string;
      blockNumber: number;
    }

    const txsWithMetering: TransactionWithMetering[] = validBlocks.flatMap(
      (block) =>
        block.data.transactions
          .filter(
            (tx): tx is typeof tx & { executionTimeUs: number } =>
              tx.executionTimeUs !== null,
          )
          .map((tx) => ({
            executionTimeUs: tx.executionTimeUs,
            gasUsed: tx.gasUsed,
            txHash: tx.hash,
            blockNumber: block.number,
          })),
    );

    if (txsWithMetering.length === 0) {
      return NextResponse.json(
        {
          timeWindowStart: 0,
          timeWindowEnd: 0,
          blockCount: validBlocks.length,
          transactionCount: 0,
          stats: {
            avgExecutionTimeUs: 0,
            minExecutionTime: null,
            maxExecutionTime: null,
            p50ExecutionTimeUs: 0,
            p95ExecutionTimeUs: 0,
            p99ExecutionTimeUs: 0,
            avgGasEfficiency: 0,
          },
        } as MeteringStatsResponse,
        {
          headers: {
            "Cache-Control": "public, s-maxage=30, stale-while-revalidate=60",
          },
        },
      );
    }

    const timestamps = validBlocks
      .map((b) => Number(b.data.timestamp))
      .filter((t) => t > 0);
    const timeWindowStart = timestamps.length > 0 ? Math.min(...timestamps) : 0;
    const timeWindowEnd = timestamps.length > 0 ? Math.max(...timestamps) : 0;

    const executionTimes = txsWithMetering.map((tx) => tx.executionTimeUs);
    const sortedExecutionTimes = [...executionTimes].sort((a, b) => a - b);

    const avgExecutionTimeUs =
      executionTimes.reduce((sum, time) => sum + time, 0) /
      executionTimes.length;

    let minTx = txsWithMetering[0];
    let maxTx = txsWithMetering[0];
    for (const tx of txsWithMetering) {
      if (tx.executionTimeUs < minTx.executionTimeUs) minTx = tx;
      if (tx.executionTimeUs > maxTx.executionTimeUs) maxTx = tx;
    }

    const p50ExecutionTimeUs = calculatePercentile(sortedExecutionTimes, 0.5);
    const p95ExecutionTimeUs = calculatePercentile(sortedExecutionTimes, 0.95);
    const p99ExecutionTimeUs = calculatePercentile(sortedExecutionTimes, 0.99);

    const gasEfficiencies = txsWithMetering
      .filter((tx) => tx.gasUsed > 0n)
      .map((tx) => tx.executionTimeUs / Number(tx.gasUsed));
    const avgGasEfficiency =
      gasEfficiencies.length > 0
        ? gasEfficiencies.reduce((sum, eff) => sum + eff, 0) /
          gasEfficiencies.length
        : 0;

    const response: MeteringStatsResponse = {
      timeWindowStart,
      timeWindowEnd,
      blockCount: validBlocks.length,
      transactionCount: txsWithMetering.length,
      stats: {
        avgExecutionTimeUs,
        minExecutionTime: {
          timeUs: minTx.executionTimeUs,
          txHash: minTx.txHash,
          blockNumber: minTx.blockNumber,
        },
        maxExecutionTime: {
          timeUs: maxTx.executionTimeUs,
          txHash: maxTx.txHash,
          blockNumber: maxTx.blockNumber,
        },
        p50ExecutionTimeUs,
        p95ExecutionTimeUs,
        p99ExecutionTimeUs,
        avgGasEfficiency,
      },
    };

    return NextResponse.json(response, {
      headers: {
        "Cache-Control": "public, s-maxage=30, stale-while-revalidate=60",
      },
    });
  } catch (error) {
    console.error("Error fetching metering stats:", error);
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 },
    );
  }
}
