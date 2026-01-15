"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { BundleHistoryResponse } from "@/app/api/bundle/[uuid]/route";
import type { BundleTransaction, MeterBundleResponse } from "@/lib/s3";

const WEI_PER_GWEI = 10n ** 9n;
const WEI_PER_ETH = 10n ** 18n;
const BLOCK_EXPLORER_URL = process.env.NEXT_PUBLIC_BLOCK_EXPLORER_URL;

interface PageProps {
  params: Promise<{ uuid: string }>;
}

function formatBigInt(value: bigint, decimals: number, scale: bigint): string {
  const whole = value / scale;
  const frac = ((value % scale) * 10n ** BigInt(decimals)) / scale;
  return `${whole}.${frac.toString().padStart(decimals, "0")}`;
}

function formatHexValue(hex: string | undefined): string {
  if (!hex) return "—";
  const value = BigInt(hex);
  if (value >= WEI_PER_ETH / 10000n) {
    return `${formatBigInt(value, 6, WEI_PER_ETH)} ETH`;
  }
  if (value >= WEI_PER_GWEI / 100n) {
    return `${formatBigInt(value, 4, WEI_PER_GWEI)} Gwei`;
  }
  return `${value.toString()} Wei`;
}

function formatGasPrice(hex: string | undefined): string {
  if (!hex) return "—";
  const value = BigInt(hex);
  return `${formatBigInt(value, 2, WEI_PER_GWEI)} Gwei`;
}

function ExplorerLink({
  type,
  value,
  children,
  className = "",
}: {
  type: "tx" | "address";
  value: string;
  children: React.ReactNode;
  className?: string;
}) {
  if (!BLOCK_EXPLORER_URL) {
    return <span className={className}>{children}</span>;
  }

  const path = type === "tx" ? `/tx/${value}` : `/address/${value}`;
  return (
    <a
      href={`${BLOCK_EXPLORER_URL}${path}`}
      target="_blank"
      rel="noopener noreferrer"
      className={`hover:underline ${className}`}
    >
      {children}
    </a>
  );
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

function Badge({
  children,
  variant = "default",
}: {
  children: React.ReactNode;
  variant?: "default" | "success" | "warning" | "error";
}) {
  const variants = {
    default: "bg-blue-50 text-blue-700 ring-blue-600/20",
    success: "bg-emerald-50 text-emerald-700 ring-emerald-600/20",
    warning: "bg-amber-50 text-amber-700 ring-amber-600/20",
    error: "bg-red-50 text-red-700 ring-red-600/20",
  };

  return (
    <span
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ring-1 ring-inset ${variants[variant]}`}
    >
      {children}
    </span>
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

function TransactionDetails({
  tx,
  index,
}: {
  tx: BundleTransaction;
  index: number;
}) {
  const [expanded, setExpanded] = useState(index === 0);

  return (
    <div className="border border-gray-200 rounded-xl overflow-hidden bg-white">
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="w-full px-5 py-4 flex items-center justify-between hover:bg-gray-50 transition-colors"
      >
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-gray-100 flex items-center justify-center text-sm font-medium text-gray-600">
            {index + 1}
          </div>
          <div className="text-left">
            <div className="flex items-center gap-2">
              <span className="font-mono text-sm text-gray-900">
                {tx.hash.slice(0, 10)}...{tx.hash.slice(-8)}
              </span>
            </div>
            <div className="text-xs text-gray-500 mt-0.5">
              {tx.signer.slice(0, 6)}...{tx.signer.slice(-4)} →{" "}
              {tx.to.slice(0, 6)}...{tx.to.slice(-4)}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <div className="text-right">
            <div className="text-sm font-semibold text-gray-900">
              {parseInt(tx.gas, 16).toLocaleString()} gas
            </div>
            <div className="text-xs text-gray-500">
              {formatHexValue(tx.value)}
            </div>
          </div>
          <svg
            className={`w-5 h-5 text-gray-400 transition-transform ${expanded ? "rotate-180" : ""}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <title>{expanded ? "Collapse" : "Expand"}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M19 9l-7 7-7-7"
            />
          </svg>
        </div>
      </button>

      {expanded && (
        <>
          <div className="px-5 pb-4 border-t border-gray-100">
            <table className="w-full text-xs">
              <tbody>
                <tr className="border-b border-gray-50">
                  <td className="text-gray-500 py-2 w-20">Hash</td>
                  <td className="py-2 text-right">
                    <span className="inline-flex items-center gap-1">
                      <ExplorerLink
                        type="tx"
                        value={tx.hash}
                        className="font-mono text-blue-600"
                      >
                        {tx.hash}
                      </ExplorerLink>
                      <CopyButton text={tx.hash} />
                    </span>
                  </td>
                </tr>
                <tr className="border-b border-gray-50">
                  <td className="text-gray-500 py-2">From</td>
                  <td className="py-2 text-right">
                    <span className="inline-flex items-center gap-1">
                      <ExplorerLink
                        type="address"
                        value={tx.signer}
                        className="font-mono text-gray-900"
                      >
                        {tx.signer}
                      </ExplorerLink>
                      <CopyButton text={tx.signer} />
                    </span>
                  </td>
                </tr>
                <tr>
                  <td className="text-gray-500 py-2">To</td>
                  <td className="py-2 text-right">
                    <span className="inline-flex items-center gap-1">
                      <ExplorerLink
                        type="address"
                        value={tx.to}
                        className="font-mono text-gray-900"
                      >
                        {tx.to}
                      </ExplorerLink>
                      <CopyButton text={tx.to} />
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div className="border-t border-gray-100 px-5 py-3 bg-gray-50/50 grid grid-cols-4 gap-4 text-xs">
            <div>
              <span className="text-gray-500">Nonce</span>
              <span className="ml-2 font-medium text-gray-900">
                {parseInt(tx.nonce, 16)}
              </span>
            </div>
            <div>
              <span className="text-gray-500">Max Fee</span>
              <span className="ml-2 font-medium text-gray-900">
                {formatGasPrice(tx.maxFeePerGas)}
              </span>
            </div>
            <div>
              <span className="text-gray-500">Priority Fee</span>
              <span className="ml-2 font-medium text-gray-900">
                {formatGasPrice(tx.maxPriorityFeePerGas)}
              </span>
            </div>
            <div>
              <span className="text-gray-500">Type</span>
              <span className="ml-2 font-medium text-gray-900">
                {tx.type === "0x2" ? "EIP-1559" : tx.type}
              </span>
            </div>
          </div>
        </>
      )}
    </div>
  );
}

function SimulationCard({ meter }: { meter: MeterBundleResponse }) {
  return (
    <Card>
      <div className="p-5">
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-6">
          <div>
            <div className="text-xs text-gray-500 mb-1">Total Gas</div>
            <div className="text-xl font-semibold text-gray-900">
              {meter.totalGasUsed.toLocaleString()}
            </div>
          </div>
          <div>
            <div className="text-xs text-gray-500 mb-1">Execution Time</div>
            <div className="text-xl font-semibold text-gray-900">
              {meter.results.reduce((sum, r) => sum + r.executionTimeUs, 0)}μs
            </div>
          </div>
          <div>
            <div className="text-xs text-gray-500 mb-1">Gas Price</div>
            <div className="text-xl font-semibold text-gray-900">
              {formatGasPrice(meter.bundleGasPrice)}
            </div>
          </div>
          <div>
            <div className="text-xs text-gray-500 mb-1">Coinbase Diff</div>
            <div className="text-xl font-semibold text-gray-900">
              {formatHexValue(meter.coinbaseDiff)}
            </div>
          </div>
        </div>
      </div>
      <div className="border-t border-gray-100 px-5 py-3 bg-gray-50/50 grid grid-cols-3 gap-4 text-xs">
        <div>
          <span className="text-gray-500">State Block</span>
          <span className="ml-2 font-medium text-gray-900">
            #{meter.stateBlockNumber}
          </span>
        </div>
        <div>
          <span className="text-gray-500">Gas Fees</span>
          <span className="ml-2 font-medium text-gray-900">
            {formatHexValue(meter.gasFees)}
          </span>
        </div>
        <div>
          <span className="text-gray-500">ETH to Coinbase</span>
          <span className="ml-2 font-medium text-gray-900">
            {formatHexValue(meter.ethSentToCoinbase)}
          </span>
        </div>
      </div>
    </Card>
  );
}

function TimelineEventDetails({
  event,
}: {
  event: BundleHistoryResponse["history"][0];
}) {
  if (event.event === "BlockIncluded" && event.data?.block_hash) {
    return (
      <div className="flex items-center gap-2">
        <Badge variant="success">{event.event}</Badge>
        <Link
          href={`/block/${event.data.block_hash}`}
          className="text-xs font-mono text-blue-600 hover:underline"
        >
          Block #{event.data.block_number}
        </Link>
      </div>
    );
  }

  if (event.event === "BuilderIncluded" && event.data?.builder) {
    return (
      <div className="flex items-center gap-2">
        <Badge>{event.event}</Badge>
        <span className="text-xs text-gray-500">
          {event.data.builder} (flashblock #{event.data.flashblock_index})
        </span>
      </div>
    );
  }

  if (event.event === "Dropped" && event.data?.reason) {
    return (
      <div className="flex items-center gap-2">
        <Badge variant="error">{event.event}</Badge>
        <span className="text-xs text-gray-500">{event.data.reason}</span>
      </div>
    );
  }

  return <Badge>{event.event}</Badge>;
}

function Timeline({ events }: { events: BundleHistoryResponse["history"] }) {
  if (events.length === 0) return null;

  return (
    <div className="divide-y divide-gray-100">
      {events.map((event, index) => (
        <div
          key={`${event.data?.key}-${index}`}
          className="flex items-center gap-4 py-3 first:pt-0 last:pb-0"
        >
          <div className="flex items-center justify-center w-6 h-6 rounded-full bg-blue-100 shrink-0">
            <div className="w-2 h-2 rounded-full bg-blue-600" />
          </div>
          <div className="flex-1 flex items-center justify-between gap-4">
            <TimelineEventDetails event={event} />
            <time className="text-sm text-gray-500 tabular-nums">
              {event.data?.timestamp
                ? new Date(event.data.timestamp).toLocaleString()
                : "—"}
            </time>
          </div>
        </div>
      ))}
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-base font-semibold text-gray-900 mb-4">{children}</h2>
  );
}

export default function BundlePage({ params }: PageProps) {
  const [uuid, setUuid] = useState<string>("");
  const [data, setData] = useState<BundleHistoryResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    params.then((p) => setUuid(p.uuid));
  }, [params]);

  useEffect(() => {
    if (!uuid) return;

    const fetchData = async () => {
      try {
        const response = await fetch(`/api/bundle/${uuid}`);
        if (!response.ok) {
          setError(
            response.status === 404
              ? "Bundle not found"
              : "Failed to fetch bundle data",
          );
          setData(null);
          return;
        }
        setData(await response.json());
        setError(null);
      } catch {
        setError("Failed to fetch bundle data");
        setData(null);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [uuid]);

  if (!uuid || (loading && !data)) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="flex items-center gap-3">
          <div className="w-5 h-5 border-2 border-blue-600 border-t-transparent rounded-full animate-spin" />
          <span className="text-gray-600">Loading bundle...</span>
        </div>
      </div>
    );
  }

  const latestBundle = data?.history
    .filter((e) => e.data?.bundle)
    .map((e) => e.data.bundle)
    .pop();

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
              {uuid}
            </code>
            <CopyButton text={uuid} />
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

        {data && latestBundle && (
          <div className="space-y-8">
            <section>
              <SectionTitle>Transactions</SectionTitle>
              <div className="space-y-3">
                {latestBundle.txs.map((tx, index) => (
                  <TransactionDetails key={tx.hash} tx={tx} index={index} />
                ))}
              </div>
            </section>

            {latestBundle.meter_bundle_response && (
              <section>
                <SectionTitle>Simulation Results</SectionTitle>
                <SimulationCard meter={latestBundle.meter_bundle_response} />
              </section>
            )}

            <section>
              <SectionTitle>Event History</SectionTitle>
              <Card className="p-6">
                {data.history.length > 0 ? (
                  <Timeline events={data.history} />
                ) : (
                  <div className="text-center py-8 text-gray-500">
                    No events recorded yet.
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
