"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { BundleHistoryResponse } from "@/app/api/bundle/[uuid]/route";

interface PageProps {
  params: Promise<{ uuid: string }>;
}

function formatEventType(eventType: string): string {
  switch (eventType) {
    case "ReceivedBundle":
      return "Bundle Received";
    case "CancelledBundle":
      return "Bundle Cancelled";
    case "BuilderMined":
      return "Builder Mined";
    case "FlashblockInclusion":
      return "Flashblock Inclusion";
    case "BlockInclusion":
      return "Block Inclusion";
    default:
      return eventType;
  }
}

function getEventStatus(eventType: string): { color: string; bgColor: string } {
  switch (eventType) {
    case "ReceivedBundle":
      return { color: "text-blue-600", bgColor: "bg-blue-100" };
    case "CancelledBundle":
      return { color: "text-red-600", bgColor: "bg-red-100" };
    case "BuilderMined":
      return { color: "text-yellow-600", bgColor: "bg-yellow-100" };
    case "FlashblockInclusion":
      return { color: "text-purple-600", bgColor: "bg-purple-100" };
    case "BlockInclusion":
      return { color: "text-green-600", bgColor: "bg-green-100" };
    default:
      return { color: "text-gray-600", bgColor: "bg-gray-100" };
  }
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
            const allTransactions = new Set<string>();

            data.history.forEach((event) => {
              if (event.event === "Created") {
                event.data?.bundle?.revertingTxHashes?.forEach((tx) => {
                  allTransactions.add(tx);
                });
              }
            });

            const uniqueTransactions = Array.from(allTransactions.values());

            return uniqueTransactions.length > 0 ? (
              <div className="border rounded-lg p-4 bg-white/5">
                <h2 className="text-xl font-semibold mb-3">
                  Transactions
                </h2>
                <ul className="space-y-2">
                  {uniqueTransactions.map((tx) => (
                    <li key={tx}>
                        {tx}
                    </li>
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
                  const { color, bgColor } = getEventStatus(event.event);
                  return (
                    <div
                      key={`${event.data?.key}-${index}`}
                      className="border rounded-lg p-4 bg-white/5"
                    >
                      <div className="flex items-start justify-between mb-2">
                        <div className="flex flex-col gap-1">
                          <span
                            className={`px-2 py-1 rounded text-sm font-medium ${color} ${bgColor}`}
                          >
                            {formatEventType(event.event)}
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

                      <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-sm">
                        {event.builder && (
                          <div>
                            <span className="font-medium">Builder:</span>{" "}
                            {event.builder}
                          </div>
                        )}
                        {event.blockNumber && (
                          <div>
                            <span className="font-medium">Block Number:</span>{" "}
                            {event.blockNumber}
                          </div>
                        )}
                        {event.flashblockIndex && (
                          <div>
                            <span className="font-medium">
                              Flashblock Index:
                            </span>{" "}
                            {event.flashblockIndex}
                          </div>
                        )}
                        {event.blockHash && (
                          <div>
                            <span className="font-medium">Block Hash:</span>{" "}
                            <span className="font-mono text-xs">
                              {event.blockHash}
                            </span>
                          </div>
                        )}
                        {event.reason && (
                          <div>
                            <span className="font-medium">Reason:</span>{" "}
                            {event.reason}
                          </div>
                        )}
                        {event.bundle?.transactions && (
                          <div className="md:col-span-2">
                            <span className="font-medium">Transactions:</span>{" "}
                            {event.bundle.transactions.length}
                            <div className="mt-2 space-y-1">
                              {event.bundle.transactions
                                .slice(0, 3)
                                .map((tx, _txIndex) => (
                                  <div
                                    key={tx.hash}
                                    className="flex items-center gap-2"
                                  >
                                    <Link
                                      href={`/txn/${tx.hash}`}
                                      className="font-mono text-xs text-blue-600 hover:text-blue-800 underline"
                                    >
                                      {tx.hash}
                                    </Link>
                                  </div>
                                ))}
                              {event.bundle.transactions.length > 3 && (
                                <div className="text-xs text-gray-500">
                                  ... and {event.bundle.transactions.length - 3}{" "}
                                  more
                                </div>
                              )}
                            </div>
                          </div>
                        )}
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
