"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

export default function Home() {
  const router = useRouter();
  const [searchHash, setSearchHash] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    const hash = searchHash.trim();
    if (!hash) return;

    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`/api/txn/${hash}`);
      if (!response.ok) {
        if (response.status === 404) {
          setError("Transaction not found");
        } else {
          setError("Failed to fetch transaction data");
        }
        return;
      }
      const result = await response.json();

      if (result.bundle_ids && result.bundle_ids.length > 0) {
        router.push(`/bundles/${result.bundle_ids[0]}`);
      } else {
        setError("No bundle found for this transaction");
      }
    } catch (_err) {
      setError("Failed to fetch transaction data");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col items-center justify-center min-h-screen p-8">
      <div className="flex flex-col items-center gap-6 w-full max-w-xl">
        <h1 className="text-3xl font-bold">TIPS</h1>
        <p className="text-gray-600 dark:text-gray-400 text-center">
          Transaction Inclusion Prioritization Stack
        </p>
        <form onSubmit={handleSearch} className="w-full flex flex-col gap-4">
          <input
            type="text"
            placeholder="Search by transaction hash..."
            value={searchHash}
            onChange={(e) => setSearchHash(e.target.value)}
            className="w-full px-4 py-3 border rounded-lg bg-white/5 border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent placeholder-gray-500 dark:placeholder-gray-400 text-center"
            disabled={loading}
          />
          <button
            type="submit"
            disabled={loading || !searchHash.trim()}
            className="px-4 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {loading ? "Searching..." : "Search"}
          </button>
        </form>
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">{error}</div>
        )}
      </div>
    </div>
  );
}
