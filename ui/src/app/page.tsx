"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import type { BlockSummary, BlocksResponse } from "./api/blocks/route";

function SearchBar({ onError }: { onError: (error: string | null) => void }) {
  const router = useRouter();
  const [searchHash, setSearchHash] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    const hash = searchHash.trim();
    if (!hash) return;

    setLoading(true);
    onError(null);

    try {
      const response = await fetch(`/api/txn/${hash}`);
      if (!response.ok) {
        if (response.status === 404) {
          onError("Transaction not found");
        } else {
          onError("Failed to fetch transaction data");
        }
        return;
      }
      const result = await response.json();

      if (result.bundle_ids && result.bundle_ids.length > 0) {
        router.push(`/bundles/${result.bundle_ids[0]}`);
      } else {
        onError("No bundle found for this transaction");
      }
    } catch {
      onError("Failed to fetch transaction data");
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSearch} className="flex items-center gap-2">
      <input
        type="text"
        placeholder="Search by transaction hash..."
        value={searchHash}
        onChange={(e) => setSearchHash(e.target.value)}
        className="w-64 lg:w-80 px-3 py-1.5 text-sm border rounded-lg bg-white border-gray-300 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent placeholder-gray-400"
        disabled={loading}
      />
      <button
        type="submit"
        disabled={loading || !searchHash.trim()}
        className="px-3 py-1.5 text-sm bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
      >
        {loading ? "..." : "Search"}
      </button>
    </form>
  );
}

function BlockRow({ block, index }: { block: BlockSummary; index: number }) {
  const opacity = Math.max(0.3, 1 - index * 0.08);
  const timeSince = Math.floor(Date.now() / 1000 - block.timestamp);
  const timeAgo =
    timeSince <= 0
      ? "now"
      : timeSince < 60
        ? `${timeSince}s ago`
        : timeSince < 3600
          ? `${Math.floor(timeSince / 60)}m ago`
          : `${Math.floor(timeSince / 3600)}h ago`;

  return (
    <Link
      href={`/block/${block.hash}`}
      className="flex items-center gap-4 px-4 py-3 border-b border-gray-100 last:border-b-0 hover:bg-gray-50 transition-all"
      style={{ opacity }}
    >
      <div className="w-10 h-10 rounded-lg bg-blue-100 flex items-center justify-center shrink-0">
        <svg
          className="w-5 h-5 text-blue-600"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <title>Block</title>
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
          />
        </svg>
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="font-semibold text-gray-900">
            #{block.number.toLocaleString()}
          </span>
          <span className="text-xs text-gray-400">{timeAgo}</span>
        </div>
        <div className="font-mono text-xs text-gray-500 truncate">
          {block.hash}
        </div>
      </div>
      <div className="text-right shrink-0">
        <div className="text-sm font-medium text-gray-900">
          {block.transactionCount}
        </div>
        <div className="text-xs text-gray-500">txns</div>
      </div>
      <svg
        className="w-4 h-4 text-gray-400 shrink-0"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <title>View</title>
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M9 5l7 7-7 7"
        />
      </svg>
    </Link>
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

export default function Home() {
  const [error, setError] = useState<string | null>(null);
  const [blocks, setBlocks] = useState<BlockSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchBlocks = async () => {
      try {
        const response = await fetch("/api/blocks");
        if (response.ok) {
          const data: BlocksResponse = await response.json();
          setBlocks(data.blocks);
        }
      } catch {
        console.error("Failed to fetch blocks");
      } finally {
        setLoading(false);
      }
    };

    fetchBlocks();
    const interval = setInterval(fetchBlocks, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-gray-50">
      <header className="bg-white border-b border-gray-200 sticky top-0 z-10">
        <div className="max-w-5xl mx-auto px-6 h-14 flex items-center justify-between">
          <span className="font-semibold text-gray-900">TIPS</span>
          <SearchBar onError={setError} />
        </div>
      </header>

      <main className="max-w-5xl mx-auto px-6 py-8">
        {error && (
          <Card className="p-4 mb-6 border-red-200 bg-red-50">
            <div className="flex items-center gap-3">
              <svg
                className="w-5 h-5 text-red-600 shrink-0"
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
              <span className="text-sm text-red-700">{error}</span>
              <button
                type="button"
                onClick={() => setError(null)}
                className="ml-auto text-red-600 hover:text-red-800"
              >
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <title>Dismiss</title>
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>
          </Card>
        )}

        <section>
          <h2 className="text-base font-semibold text-gray-900 mb-4">
            Latest Blocks
          </h2>

          <Card className="overflow-hidden">
            {loading ? (
              <div className="py-12 flex items-center justify-center">
                <div className="flex items-center gap-3">
                  <div className="w-5 h-5 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" />
                  <span className="text-gray-600">Loading blocks...</span>
                </div>
              </div>
            ) : blocks.length > 0 ? (
              <div className="divide-y divide-gray-100">
                {blocks.map((block, index) => (
                  <BlockRow key={block.hash} block={block} index={index} />
                ))}
              </div>
            ) : (
              <div className="py-12 text-center text-gray-500">
                No blocks available
              </div>
            )}
          </Card>
        </section>
      </main>
    </div>
  );
}
