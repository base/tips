"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { BlockData, BlockTransaction } from "@/lib/s3";

const BLOCK_EXPLORER_URL = process.env.NEXT_PUBLIC_BLOCK_EXPLORER_URL;

interface PageProps {
  params: Promise<{ hash: string }>;
}

function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <button
      type="button"
      onClick={handleCopy}
      className="p-1.5 rounded-md hover:bg-gray-100 transition-colors text-gray-400 hover:text-gray-600"
      title="Copy to clipboard"
    >
      {copied ? (
        <svg
          className="w-4 h-4 text-green-500"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>Copied</title>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M5 13l4 4L19 7"
          />
        </svg>
      ) : (
        <svg
          className="w-4 h-4"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>Copy</title>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"
          />
        </svg>
      )}
    </button>
  );
}

function Card({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`bg-white rounded-xl border border-gray-200 shadow-sm ${className}`}
    >
      {children}
    </div>
  );
}

function getHeatmapStyle(
  executionTimeUs: number,
  maxTime: number,
): { bg: string; text: string } {
  if (maxTime === 0) return { bg: "bg-amber-50", text: "text-amber-700" };
  const ratio = Math.min(executionTimeUs / maxTime, 1);
  if (ratio < 0.2) return { bg: "bg-amber-100", text: "text-amber-800" };
  if (ratio < 0.4) return { bg: "bg-amber-200", text: "text-amber-900" };
  if (ratio < 0.6) return { bg: "bg-orange-200", text: "text-orange-900" };
  if (ratio < 0.8) return { bg: "bg-orange-300", text: "text-orange-950" };
  return { bg: "bg-red-300", text: "text-red-950" };
}

function TransactionRow({
  tx,
  maxExecutionTime,
}: {
  tx: BlockTransaction;
  maxExecutionTime: number;
}) {
  const hasBundle = tx.bundleId !== null;
  const hasExecutionTime = tx.executionTimeUs !== null;
  const executionTime = tx.executionTimeUs ?? 0;
  const heatmapStyle = hasExecutionTime
    ? getHeatmapStyle(executionTime, maxExecutionTime)
    : null;

  const content = (
    <div
      className={`flex items-center gap-4 px-4 py-3 border-b border-gray-100 last:border-b-0 transition-colors bg-white hover:bg-gray-50 ${hasBundle ? "cursor-pointer" : ""}`}
    >
      <div className="w-12 text-center">
        <span className="text-xs font-medium text-gray-500">{tx.index}</span>
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          {BLOCK_EXPLORER_URL ? (
            <a
              href={`${BLOCK_EXPLORER_URL}/tx/${tx.hash}`}
              target="_blank"
              rel="noopener noreferrer"
              className="font-mono text-sm text-blue-600 hover:underline break-all"
              onClick={(e) => e.stopPropagation()}
            >
              {tx.hash}
            </a>
          ) : (
            <span className="font-mono text-sm text-gray-900 break-all">
              {tx.hash}
            </span>
          )}
        </div>
        <div className="text-xs text-gray-500 mt-0.5">
          {tx.from.slice(0, 6)}...{tx.from.slice(-4)}
          {tx.to && (
            <>
              {" → "}
              {tx.to.slice(0, 6)}...{tx.to.slice(-4)}
            </>
          )}
        </div>
      </div>
      <div className="text-right">
        {hasExecutionTime && heatmapStyle ? (
          <span
            className={`inline-block px-2 py-0.5 rounded text-sm font-medium ${heatmapStyle.bg} ${heatmapStyle.text}`}
          >
            {executionTime.toLocaleString()}μs
          </span>
        ) : (
          <div className="text-sm font-medium text-gray-400">—</div>
        )}
        <div className="text-xs text-gray-500 mt-0.5">
          {tx.gasUsed.toLocaleString()} gas
        </div>
      </div>
    </div>
  );

  if (hasBundle) {
    return <Link href={`/bundles/${tx.bundleId}`}>{content}</Link>;
  }

  return content;
}

function BlockStats({ block }: { block: BlockData }) {
  const txsWithTime = block.transactions.filter(
    (tx) => tx.executionTimeUs !== null,
  );
  const totalExecutionTime = txsWithTime.reduce(
    (sum, tx) => sum + (tx.executionTimeUs ?? 0),
    0,
  );
  const bundleCount = block.transactions.filter(
    (tx) => tx.bundleId !== null,
  ).length;

  return (
    <Card>
      <div className="p-5">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-6">
          <div>
            <div className="text-xs text-gray-500 mb-1">Block Number</div>
            <div className="text-xl font-semibold text-gray-900">
              #{block.number.toLocaleString()}
            </div>
          </div>
          <div>
            <div className="text-xs text-gray-500 mb-1">Transactions</div>
            <div className="text-xl font-semibold text-gray-900">
              {block.transactions.length}
            </div>
          </div>
          <div>
            <div className="text-xs text-gray-500 mb-1">Bundles</div>
            <div className="text-xl font-semibold text-gray-900">
              {bundleCount}
            </div>
          </div>
          <div>
            <div className="text-xs text-gray-500 mb-1">Total Exec Time</div>
            <div className="text-xl font-semibold text-gray-900">
              {totalExecutionTime > 0
                ? `${totalExecutionTime.toLocaleString()}μs`
                : "—"}
            </div>
          </div>
        </div>
      </div>
      <div className="border-t border-gray-100 px-5 py-3 bg-gray-50/50 grid grid-cols-3 gap-4 text-xs">
        <div>
          <span className="text-black">Gas Used</span>
          <span className="ml-2 font-medium text-black">
            {block.gasUsed.toLocaleString()}
          </span>
        </div>
        <div>
          <span className="text-black">Gas Limit</span>
          <span className="ml-2 font-medium text-black">
            {block.gasLimit.toLocaleString()}
          </span>
        </div>
        <div>
          <span className="text-black">Timestamp</span>
          <span className="ml-2 font-medium text-black">
            {new Date(Number(block.timestamp) * 1000).toLocaleString()}
          </span>
        </div>
      </div>
    </Card>
  );
}

export default function BlockPage({ params }: PageProps) {
  const [hash, setHash] = useState<string>("");
  const [data, setData] = useState<BlockData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    params.then((p) => setHash(p.hash));
  }, [params]);

  useEffect(() => {
    if (!hash) return;

    const fetchData = async () => {
      try {
        const response = await fetch(`/api/block/${hash}`);
        if (!response.ok) {
          setError(
            response.status === 404
              ? "Block not found"
              : "Failed to fetch block data",
          );
          setData(null);
          return;
        }
        setData(await response.json());
        setError(null);
      } catch {
        setError("Failed to fetch block data");
        setData(null);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [hash]);

  if (!hash || loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-5 h-5 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" />
          <span className="text-gray-600">Loading block...</span>
        </div>
      </div>
    );
  }

  const maxExecutionTime = data
    ? Math.max(
        ...data.transactions
          .filter((tx) => tx.executionTimeUs !== null)
          .map((tx) => tx.executionTimeUs ?? 0),
        0,
      )
    : 0;

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b border-gray-200 sticky top-0 z-10">
        <div className="max-w-5xl mx-auto px-6 h-14 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <Link
              href="/"
              className="inline-flex items-center justify-center w-8 h-8 rounded-lg hover:bg-gray-100 transition-colors text-gray-500 hover:text-gray-900"
              title="Back to search"
            >
              <svg
                className="w-5 h-5"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <title>Back</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M15 19l-7-7 7-7"
                />
              </svg>
            </Link>
            <div className="h-6 w-px bg-gray-200" />
            <Link
              href="/"
              className="font-semibold text-gray-900 hover:text-blue-600 transition-colors"
            >
              TIPS
            </Link>
          </div>
          <div className="flex items-center gap-2 text-sm">
            <code className="font-mono text-gray-600 bg-gray-100 px-2 py-1 rounded text-xs">
              {hash.slice(0, 10)}...{hash.slice(-8)}
            </code>
            <CopyButton text={hash} />
            {BLOCK_EXPLORER_URL && (
              <a
                href={`${BLOCK_EXPLORER_URL}/block/${hash}`}
                target="_blank"
                rel="noopener noreferrer"
                className="p-1.5 rounded-md hover:bg-gray-100 transition-colors text-gray-400 hover:text-gray-600"
                title="View on block explorer"
              >
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <title>External link</title>
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                  />
                </svg>
              </a>
            )}
          </div>
        </div>
      </header>

      <main className="max-w-5xl mx-auto px-6 py-8">
        {error && (
          <Card className="p-6 mb-8 border-red-200 bg-red-50">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 rounded-full bg-red-100 flex items-center justify-center">
                <svg
                  className="w-5 h-5 text-red-600"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <title>Error</title>
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </div>
              <div>
                <h3 className="font-medium text-red-900">Error</h3>
                <p className="text-sm text-red-700">{error}</p>
              </div>
            </div>
          </Card>
        )}

        {data && (
          <div className="space-y-8">
            <section>
              <h2 className="text-base font-semibold text-gray-900 mb-4">
                Block Overview
              </h2>
              <BlockStats block={data} />
            </section>

            <section>
              <h2 className="text-base font-semibold text-gray-900 mb-4">
                Transactions
              </h2>
              <Card className="overflow-hidden">
                <div className="px-4 py-2 bg-gray-50 border-b border-gray-200 flex items-center gap-4 text-xs font-medium text-gray-500">
                  <div className="w-12 text-center">#</div>
                  <div className="flex-1">Transaction</div>
                  <div className="text-right">Execution</div>
                </div>
                <div className="divide-y divide-gray-100 bg-white">
                  {data.transactions.map((tx) => (
                    <TransactionRow
                      key={tx.hash}
                      tx={tx}
                      maxExecutionTime={maxExecutionTime}
                    />
                  ))}
                </div>
                {data.transactions.length === 0 && (
                  <div className="py-12 text-center text-gray-500">
                    No transactions in this block
                  </div>
                )}
              </Card>
            </section>
          </div>
        )}
      </main>
    </div>
  );
}
