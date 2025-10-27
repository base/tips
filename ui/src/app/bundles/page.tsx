"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";

export default function BundlesPage() {
  const [allBundles, setAllBundles] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchHash, setSearchHash] = useState<string>("");
  const [filteredAllBundles, setFilteredAllBundles] = useState<string[]>([]);
  const debounceTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const filterBundles = useCallback(
    async (searchTerm: string, all: string[]) => {
      if (!searchTerm.trim()) {
        setFilteredAllBundles(all);
        return;
      }

      let allBundlesWithTx: string[] = [];

      try {
        const response = await fetch(`/api/txn/${searchTerm.trim()}`);

        if (response.ok) {
          const txnData = await response.json();
          const bundleIds = txnData.bundle_ids || [];

          allBundlesWithTx = all.filter((bundleId) =>
            bundleIds.includes(bundleId),
          );
        }
      } catch (err) {
        console.error("Error filtering bundles:", err);
      }

      setFilteredAllBundles(allBundlesWithTx);
    },
    [],
  );

  useEffect(() => {
    const fetchAllBundles = async () => {
      try {
        const response = await fetch("/api/bundles");
        if (!response.ok) {
          setError("Failed to fetch bundles");
          setAllBundles([]);
          return;
        }
        const result = await response.json();
        setAllBundles(result);
        setError(null);
      } catch (_err) {
        setError("Failed to fetch bundles");
        setAllBundles([]);
      }
    };

    const fetchData = async () => {
      await fetchAllBundles();
      setLoading(false);
    };

    fetchData();

    const interval = setInterval(fetchData, 400);

    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (debounceTimeoutRef.current) {
      clearTimeout(debounceTimeoutRef.current);
    }

    if (!searchHash.trim()) {
      filterBundles(searchHash, allBundles);
    } else {
      debounceTimeoutRef.current = setTimeout(() => {
        filterBundles(searchHash, allBundles);
      }, 300);
    }

    return () => {
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
      }
    };
  }, [searchHash, allBundles, filterBundles]);

  if (loading) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <h1 className="text-2xl font-bold">BundleStore (fka Mempool)</h1>
        <div className="animate-pulse">Loading bundles...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-8 p-8">
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between">
          <h1 className="text-2xl font-bold">BundleStore (fka Mempool)</h1>
          <div className="flex items-center gap-2">
            <input
              type="text"
              placeholder="Search by transaction hash..."
              value={searchHash}
              onChange={(e) => setSearchHash(e.target.value)}
              className="px-3 py-2 border rounded-lg bg-white/5 border-gray-300 dark:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent placeholder-gray-500 dark:placeholder-gray-400 text-sm min-w-[300px]"
            />
          </div>
        </div>
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">{error}</div>
        )}
      </div>

      <div className="flex flex-col gap-6">
        <section>
          <h2 className="text-xl font-semibold mb-4">
            All Bundles
            {searchHash.trim() && (
              <span className="text-sm font-normal text-gray-500 ml-2">
                ({filteredAllBundles.length} found)
              </span>
            )}
          </h2>
          {filteredAllBundles.length > 0 ? (
            <ul className="space-y-2">
              {filteredAllBundles.map((bundleId) => (
                <li key={bundleId}>
                  <Link
                    href={`/bundles/${bundleId}`}
                    className="block p-3 border rounded-lg bg-white/5 hover:bg-white/10 transition-colors"
                  >
                    <span className="font-mono text-sm">{bundleId}</span>
                  </Link>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-gray-600 dark:text-gray-400">
              {searchHash.trim()
                ? "No bundles found matching this transaction hash."
                : "No bundles found."}
            </p>
          )}
        </section>
      </div>
    </div>
  );
}
