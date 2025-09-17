"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import type { TransactionHistoryResponse } from "@/app/api/txn/[hash]/route";

interface PageProps {
  params: Promise<{ hash: string }>;
}

export default function TransactionRedirectPage({ params }: PageProps) {
  const router = useRouter();
  const [hash, setHash] = useState<string>("");
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

    const fetchAndRedirect = async () => {
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

    fetchAndRedirect();
  }, [hash, router]);

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
        <h1 className="text-2xl font-bold">Transaction {hash}</h1>
        {loading && (
          <div className="text-sm text-gray-500">
            Redirecting to bundle page...
          </div>
        )}
        {error && (
          <div className="text-sm text-red-600 dark:text-red-400">{error}</div>
        )}
      </div>
    </div>
  );
}
