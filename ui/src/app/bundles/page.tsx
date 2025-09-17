"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { Bundle } from "@/app/api/bundles/route";

export default function BundlesPage() {
  const [bundles, setBundles] = useState<Bundle[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchBundles = async () => {
      try {
        const response = await fetch("/api/bundles");
        if (!response.ok) {
          setError("Failed to fetch bundles");
          setBundles([]);
          return;
        }
        const result = await response.json();
        setBundles(result);
        setError(null);
      } catch (_err) {
        setError("Failed to fetch bundles");
        setBundles([]);
      } finally {
        setLoading(false);
      }
    };

    fetchBundles();

    const interval = setInterval(fetchBundles, 400);

    return () => clearInterval(interval);
  }, []);

  if (loading) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <h1 className="text-2xl font-bold">Bundles</h1>
        <div className="animate-pulse">Loading bundles...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-8">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold">Bundles</h1>
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">{error}</div>
        )}
      </div>

      {bundles.length > 0 ? (
        <ul className="space-y-2">
          {bundles.map((bundle) => (
            <li key={bundle.id}>
              <Link
                href={`/bundle/${bundle.id}`}
                className="block p-3 border rounded-lg bg-white/5 hover:bg-white/10 transition-colors"
              >
                <span className="font-mono text-sm">
                  {bundle.id}
                  {" ("}
                  {bundle.txnHashes?.join(", ") || "No transactions"}
                  {")"}
                </span>
              </Link>
            </li>
          ))}
        </ul>
      ) : (
        <p className="text-gray-600 dark:text-gray-400">
          {loading ? "Loading bundles..." : "No bundles found."}
        </p>
      )}
    </div>
  );
}
