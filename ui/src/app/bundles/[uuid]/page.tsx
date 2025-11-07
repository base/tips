"use client";

import { useEffect, useState } from "react";
import type { BundleHistoryResponse } from "@/app/api/bundle/[uuid]/route";

interface PageProps {
  params: Promise<{ uuid: string }>;
}

export default function BundlePage({ params }: PageProps) {
  const [uuid, setUuid] = useState<string>("");
  const [data, setData] = useState<BundleHistoryResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const initializeParams = async () => {
      const resolvedParams = await params;
      setUuid(resolvedParams.uuid);
    };
    initializeParams();
  }, [params]);

  useEffect(() => {
    if (!uuid) return;

    const fetchData = async () => {
      try {
        const response = await fetch(`/api/bundle/${uuid}`);
        if (!response.ok) {
          if (response.status === 404) {
            setError("Bundle not found");
          } else {
            setError("Failed to fetch bundle data");
          }
          setData(null);
          return;
        }
        const result = await response.json();
        setData(result);
        setError(null);
      } catch (_err) {
        setError("Failed to fetch bundle data");
        setData(null);
      } finally {
        setLoading(false);
      }
    };

    fetchData();

    const interval = setInterval(fetchData, 400);

    return () => clearInterval(interval);
  }, [uuid]);

  if (!uuid) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <div className="animate-pulse">Loading...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-8">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold">Bundle {uuid}</h1>
        {loading && (
          <div className="text-sm text-gray-500">Loading bundle data...</div>
        )}
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">{error}</div>
        )}
      </div>

      {data && (
        <div className="flex flex-col gap-6">
          {(() => {
            const transactions = new Set<string>();

            data.history.forEach((event) => {
              event.data?.bundle?.revertingTxHashes?.forEach((tx) => {
                transactions.add(tx);
              });
            });

            return transactions.size > 0 ? (
              <div className="border rounded-lg p-4 bg-white/5">
                <h2 className="text-xl font-semibold mb-3">Transactions</h2>
                <ul className="space-y-2">
                  {Array.from(transactions).map((tx) => (
                    <li key={tx}>{tx}</li>
                  ))}
                </ul>
              </div>
            ) : null;
          })()}

          <div className="flex flex-col gap-4">
            <h2 className="text-xl font-semibold">Bundle History</h2>

            {data.history.length > 0 ? (
              <div className="space-y-4">
                {data.history.map((event, index) => {
                  return (
                    <div
                      key={`${event.data?.key}-${index}`}
                      className="border rounded-lg p-4 bg-white/5"
                    >
                      <div className="flex items-start justify-between mb-2">
                        <div className="flex flex-col gap-1">
                          <span
                            className={`px-2 py-1 rounded text-sm font-medium bg-gray-200 text-black`}
                          >
                            {event.event}
                          </span>
                          <span className="text-xs text-gray-500">
                            {event.data?.timestamp
                              ? new Date(event.data?.timestamp).toLocaleString()
                              : "No timestamp"}
                          </span>
                        </div>
                        <span className="text-xs text-gray-500">
                          Event #{index + 1}
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : (
              <p className="text-gray-600 dark:text-gray-400">
                {loading
                  ? "Loading events..."
                  : "No events found for this bundle."}
              </p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
