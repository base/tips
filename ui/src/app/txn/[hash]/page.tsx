"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import type {
  TransactionHistoryResponse,
  BundleEventWithId,
} from "@/app/api/txn/[hash]/route";

interface PageProps {
  params: Promise<{ hash: string }>;
}

// Color palette for distinguishing bundles
const BUNDLE_COLORS = [
  {
    bg: "bg-blue-100",
    dot: "bg-blue-600",
    text: "text-blue-700",
    badge: "bg-blue-50 text-blue-700 ring-blue-600/20",
    link: "bg-blue-50 text-blue-600",
  },
  {
    bg: "bg-purple-100",
    dot: "bg-purple-600",
    text: "text-purple-700",
    badge: "bg-purple-50 text-purple-700 ring-purple-600/20",
    link: "bg-purple-50 text-purple-600",
  },
  {
    bg: "bg-amber-100",
    dot: "bg-amber-600",
    text: "text-amber-700",
    badge: "bg-amber-50 text-amber-700 ring-amber-600/20",
    link: "bg-amber-50 text-amber-600",
  },
  {
    bg: "bg-emerald-100",
    dot: "bg-emerald-600",
    text: "text-emerald-700",
    badge: "bg-emerald-50 text-emerald-700 ring-emerald-600/20",
    link: "bg-emerald-50 text-emerald-600",
  },
  {
    bg: "bg-rose-100",
    dot: "bg-rose-600",
    text: "text-rose-700",
    badge: "bg-rose-50 text-rose-700 ring-rose-600/20",
    link: "bg-rose-50 text-rose-600",
  },
  {
    bg: "bg-cyan-100",
    dot: "bg-cyan-600",
    text: "text-cyan-700",
    badge: "bg-cyan-50 text-cyan-700 ring-cyan-600/20",
    link: "bg-cyan-50 text-cyan-600",
  },
];

type BundleColorMap = Map<string, (typeof BUNDLE_COLORS)[0]>;

function Badge({
  children,
  variant = "default",
  className = "",
}: {
  children: React.ReactNode;
  variant?: "default" | "success" | "warning" | "error";
  className?: string;
}) {
  const variants = {
    default: "bg-blue-50 text-blue-700 ring-blue-600/20",
    success: "bg-emerald-50 text-emerald-700 ring-emerald-600/20",
    warning: "bg-amber-50 text-amber-700 ring-amber-600/20",
    error: "bg-red-50 text-red-700 ring-red-600/20",
  };

  return (
    <span
      className={`inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium ring-1 ring-inset ${
        className || variants[variant]
      }`}
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

function TimelineEventDetails({
  event,
  colors,
}: {
  event: BundleEventWithId;
  colors: (typeof BUNDLE_COLORS)[0];
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
        <Badge className={colors.badge}>{event.event}</Badge>
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

  if (event.event === "Executed") {
    return (
      <div className="flex items-center gap-2">
        <Badge className={colors.badge}>{event.event}</Badge>
      </div>
    );
  }

  if (event.event === "BackrunBundleExecuted") {
    return (
      <div className="flex items-center gap-2">
        <Badge className={colors.badge}>{event.event}</Badge>
      </div>
    );
  }

  return <Badge className={colors.badge}>{event.event}</Badge>;
}

function Timeline({
  events,
  bundleColorMap,
}: {
  events: BundleEventWithId[];
  bundleColorMap: BundleColorMap;
}) {
  if (events.length === 0) return null;

  return (
    <div className="divide-y divide-gray-100">
      {events.map((event, index) => {
        const colors = bundleColorMap.get(event.bundleId) ?? BUNDLE_COLORS[0];
        return (
          <div
            key={`${event.data?.key}-${index}`}
            className="flex items-center gap-4 py-3 first:pt-0 last:pb-0"
          >
            <div
              className={`flex items-center justify-center w-6 h-6 rounded-full ${colors.bg} shrink-0`}
            >
              <div className={`w-2 h-2 rounded-full ${colors.dot}`} />
            </div>
            <div className="flex-1 flex items-center justify-between gap-4">
              <TimelineEventDetails event={event} colors={colors} />
              <time className="text-sm text-gray-500 tabular-nums">
                {event.data?.timestamp
                  ? formatTimestampWithMs(event.data.timestamp)
                  : "—"}
              </time>
            </div>
          </div>
        );
      })}
    </div>
  );
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-base font-semibold text-gray-900 mb-4">{children}</h2>
  );
}

function formatTimestampWithMs(timestamp: number): string {
  const date = new Date(timestamp);
  const dateStr = date.toLocaleDateString();
  const hours = date.getHours().toString().padStart(2, "0");
  const minutes = date.getMinutes().toString().padStart(2, "0");
  const seconds = date.getSeconds().toString().padStart(2, "0");
  const ms = date.getMilliseconds().toString().padStart(3, "0");
  return `${dateStr}, ${hours}:${minutes}:${seconds}.${ms}`;
}

export default function TransactionPage({ params }: PageProps) {
  const router = useRouter();
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
          return;
        }
        const result: TransactionHistoryResponse = await response.json();

        // If only 1 bundle, redirect to bundle page
        if (result.bundle_ids && result.bundle_ids.length === 1) {
          router.replace(`/bundles/${result.bundle_ids[0]}`);
          return;
        }

        setData(result);
      } catch (_err) {
        setError("Failed to fetch transaction data");
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, [hash, router]);

  if (!hash || loading) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <div className="animate-pulse">Loading...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col gap-6 p-8">
        <div className="flex flex-col gap-2">
          <h1 className="text-2xl font-bold">Transaction</h1>
          <p className="text-sm font-mono text-gray-500 break-all">{hash}</p>
          <div className="text-sm text-red-600 dark:text-red-400">{error}</div>
        </div>
      </div>
    );
  }

  if (!data) {
    return null;
  }

  // Create bundle -> color mapping
  const bundleColorMap: BundleColorMap = new Map(
    data.bundle_ids.map((id, index) => [
      id,
      BUNDLE_COLORS[index % BUNDLE_COLORS.length],
    ])
  );

  return (
    <div className="flex flex-col gap-6 p-8 max-w-4xl mx-auto">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold">Transaction</h1>
        <p className="text-sm font-mono text-gray-500 break-all">{hash}</p>
      </div>

      {/* Bundle IDs */}
      <section>
        <SectionTitle>Associated Bundles</SectionTitle>
        <Card className="p-6">
          <div className="flex flex-wrap gap-2">
            {data.bundle_ids.map((bundleId) => {
              const colors = bundleColorMap.get(bundleId) ?? BUNDLE_COLORS[0];
              return (
                <Link
                  key={bundleId}
                  href={`/bundles/${bundleId}`}
                  className={`text-xs font-mono hover:underline px-2 py-1 rounded ${colors.link}`}
                >
                  {bundleId}
                </Link>
              );
            })}
          </div>
        </Card>
      </section>

      {/* Event History */}
      <section>
        <SectionTitle>Event History</SectionTitle>
        <Card className="p-6">
          {data.history.length > 0 ? (
            <Timeline events={data.history} bundleColorMap={bundleColorMap} />
          ) : (
            <div className="text-center py-8 text-gray-500">
              No events recorded yet.
            </div>
          )}
        </Card>
      </section>
    </div>
  );
}
