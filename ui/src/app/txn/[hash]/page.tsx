"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

interface TransactionEvent {
  type: string;
  data: {
    bundle_id?: string;
    transactions?: Array<{
      id: {
        sender: string;
        nonce: string;
        hash: string;
      };
      data: string;
    }>;
    transaction_ids?: Array<{
      sender: string;
      nonce: string;
      hash: string;
    }>;
    block_number?: number;
    flashblock_index?: number;
    block_hash?: string;
  };
}

interface TransactionHistoryResponse {
  hash: string;
  events: TransactionEvent[];
  metadata?: {
    bundle_ids: string[];
    sender: string;
    nonce: string;
  };
}

interface PageProps {
  params: Promise<{ hash: string }>;
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

export default function TransactionPage({ params }: PageProps) {
  const [hash, setHash] = useState<string>("");
  const [data, setData] = useState<TransactionHistoryResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const initializeParams = async () => {
      const resolvedParams = await params;
      setHash(resolvedParams.hash);
    };
    initializeParams();
  }, [params]);

  useEffect(() => {
    if (!hash) return;

    const fetchData = async () => {
      try {
        const response = await fetch(`/api/txn/${hash}`);
        if (!response.ok) {
          if (response.status === 404) {
            setError("Transaction not found");
          } else {
            setError("Failed to fetch transaction data");
          }
          setData(null);
          return;
        }
        const result = await response.json();
        setData(result);
        setError(null);
      } catch (_err) {
        setError("Failed to fetch transaction data");
        setData(null);
      } finally {
        setLoading(false);
      }
    };

    fetchData();

    const interval = setInterval(fetchData, 100);

    return () => clearInterval(interval);
  }, [hash]);

  if (!hash) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <div className="animate-pulse">Loading...</div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-8">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold">{hash}</h1>
        {loading && (
          <div className="text-sm text-gray-500">
            Loading transaction data...
          </div>
        )}
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">{error}</div>
        )}
      </div>

      {data?.metadata && (
        <div className="border rounded-lg p-4 bg-white/5">
          <h2 className="text-xl font-semibold mb-2">Transaction Details</h2>
          <div className="text-sm text-gray-600 dark:text-gray-400 space-y-1">
            <p>
              <strong>Sender:</strong> {data.metadata.sender}
            </p>
            <p>
              <strong>Nonce:</strong> {data.metadata.nonce}
            </p>
            <p>
              <strong>Bundle IDs:</strong> {data.metadata.bundle_ids.length}
            </p>
          </div>
        </div>
      )}

      {data && (
        <div className="flex flex-col gap-4">
          <h2 className="text-xl font-semibold">
            Event Timeline ({data.events.length} events)
          </h2>

          {data.events.length > 0 ? (
            <div className="space-y-4">
              {data.events.map((event, index) => {
                const { color, bgColor } = getEventStatus(event.type);
                return (
                  <div
                    key={`${event.type}-${index}`}
                    className="border rounded-lg p-4 bg-white/5"
                  >
                    <div className="flex items-start justify-between mb-2">
                      <span
                        className={`px-2 py-1 rounded text-sm font-medium ${color} ${bgColor}`}
                      >
                        {formatEventType(event.type)}
                      </span>
                      <span className="text-xs text-gray-500">
                        Event #{index + 1}
                      </span>
                    </div>

                    <div className="grid grid-cols-1 md:grid-cols-2 gap-2 text-sm">
                      {event.data.bundle_id && (
                        <div>
                          <span className="font-medium">Bundle ID:</span>{" "}
                          <Link
                            href={`/bundle/${event.data.bundle_id}`}
                            className="text-blue-600 hover:text-blue-800 underline"
                          >
                            {event.data.bundle_id}
                          </Link>
                        </div>
                      )}

                      {event.data.block_number && (
                        <div>
                          <span className="font-medium">Block Number:</span>{" "}
                          {event.data.block_number}
                        </div>
                      )}

                      {event.data.flashblock_index !== undefined && (
                        <div>
                          <span className="font-medium">Flashblock Index:</span>{" "}
                          {event.data.flashblock_index}
                        </div>
                      )}

                      {event.data.block_hash && (
                        <div>
                          <span className="font-medium">Block Hash:</span>{" "}
                          <span className="font-mono text-xs">
                            {event.data.block_hash}
                          </span>
                        </div>
                      )}

                      {event.data.transaction_ids && (
                        <div className="md:col-span-2">
                          <span className="font-medium">Transaction IDs:</span>{" "}
                          {event.data.transaction_ids.length}
                        </div>
                      )}

                      {event.data.transactions && (
                        <div className="md:col-span-2">
                          <span className="font-medium">Transactions:</span>{" "}
                          {event.data.transactions.length}
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
                : "No events found for this transaction."}
            </p>
          )}
        </div>
      )}
    </div>
  );
}
