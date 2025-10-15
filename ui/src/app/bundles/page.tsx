"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Bundle } from "@/app/api/bundles/route";

export default function BundlesPage() {
  const [liveBundles, setLiveBundles] = useState<Bundle[]>([]);
  const [allBundles, setAllBundles] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchHash, setSearchHash] = useState<string>("");
  const [filteredLiveBundles, setFilteredLiveBundles] = useState<Bundle[]>([]);
  const [filteredAllBundles, setFilteredAllBundles] = useState<string[]>([]);
  const debounceTimeoutRef = useRef<NodeJS.Timeout | null>(null);

  const filterBundles = useCallback(
    async (searchTerm: string, live: Bundle[], all: string[]) => {
      if (!searchTerm.trim()) {
        setFilteredLiveBundles(live);
        setFilteredAllBundles(all);
        return;
      }

      // Filter live bundles immediately for better UX
      const liveBundlesWithTx = live.filter((bundle) =>
        bundle.txnHashes?.some((hash) =>
          hash.toLowerCase().includes(searchTerm.toLowerCase()),
        ),
      );

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

      // Batch all state updates together to prevent jitter
      setFilteredLiveBundles(liveBundlesWithTx);
      setFilteredAllBundles(allBundlesWithTx);
    },
    [],
  );

  useEffect(() => {
    const fetchLiveBundles = async () => {
      try {
        const response = await fetch("/api/bundles");
        if (!response.ok) {
          setError("Failed to fetch live bundles");
          setLiveBundles([]);
          return;
        }
        const result = await response.json();
        setLiveBundles(result);
        setError(null);
      } catch (_err) {
        setError("Failed to fetch live bundles");
        setLiveBundles([]);
      }
    };

    const fetchAllBundles = async () => {
      try {
        const response = await fetch("/api/bundles/all");
        if (!response.ok) {
          console.error("Failed to fetch all bundles from S3");
          setAllBundles([]);
          return;
        }
        const result = await response.json();
        setAllBundles(result);
      } catch (_err) {
        console.error("Failed to fetch all bundles from S3");
        setAllBundles([]);
      }
    };

    const fetchData = async () => {
      await Promise.all([fetchLiveBundles(), fetchAllBundles()]);
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
      // No debounce for clearing search
      filterBundles(searchHash, liveBundles, allBundles);
    } else {
      // Debounce API calls for non-empty search
      debounceTimeoutRef.current = setTimeout(() => {
        filterBundles(searchHash, liveBundles, allBundles);
      }, 300);
    }

    return () => {
      if (debounceTimeoutRef.current) {
        clearTimeout(debounceTimeoutRef.current);
      }
    };
  }, [searchHash, liveBundles, allBundles, filterBundles]);

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
            Live Bundles
            {searchHash.trim() && (
              <span className="text-sm font-normal text-gray-500 ml-2">
                ({filteredLiveBundles.length} found)
              </span>
            )}
          </h2>
          {filteredLiveBundles.length > 0 ? (
            <ul className="space-y-2">
              {filteredLiveBundles.map((bundle) => (
                <li key={bundle.id}>
                  <Link
                    href={`/bundles/${bundle.id}`}
                    className="block p-3 border rounded-lg bg-white/5 hover:bg-white/10 transition-colors"
                  >
                    <div className="flex flex-col gap-1">
                      <span className="font-mono text-sm">{bundle.id}</span>
                      <div className="flex items-center gap-2 text-xs">
                        <span
                          className={`px-2 py-1 rounded font-medium ${
                            bundle.state === "Ready"
                              ? "bg-blue-100 text-blue-600"
                              : bundle.state === "IncludedByBuilder"
                                ? "bg-green-100 text-green-600"
                                : "bg-gray-100 text-gray-600"
                          }`}
                        >
                          {bundle.state}
                        </span>
                        <span className="text-gray-500">
                          {bundle.txnHashes?.join(", ") || "No transactions"}
                        </span>
                      </div>
                    </div>
                  </Link>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-gray-600 dark:text-gray-400">
              {searchHash.trim()
                ? "No live bundles found matching this transaction hash."
                : "No live bundles found."}
            </p>
          )}
        </section>

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
                ? "No bundles found in S3 matching this transaction hash."
                : "No bundles found in S3."}
            </p>
          )}
        </section>
      </div>
    </div>
  );
}
