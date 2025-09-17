"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

export default function Home() {
  const [txnHash, setTxnHash] = useState("");
  const router = useRouter();

  const handleLookup = () => {
    if (txnHash.trim()) {
      router.push(`/txn/${txnHash.trim()}`);
    }
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleLookup();
    }
  };

  return (
    <div>
      <main className="flex flex-col gap-[32px] row-start-2 items-center sm:items-start">
        <div className="w-full max-w-4xl">
          <h1 className="text-2xl font-bold mb-8">
            Tips - Transaction Inclusion Pipeline Services
          </h1>

          <div className="mb-8 p-4 border rounded-lg bg-white/5">
            <h2 className="text-xl font-semibold mb-4">Transaction Lookup</h2>
            <div className="flex gap-2">
              <input
                type="text"
                value={txnHash}
                onChange={(e) => setTxnHash(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder="Enter transaction hash"
                className="flex-1 px-3 py-2 border rounded bg-white/10 placeholder-gray-500 text-white"
              />
              <button
                type="button"
                onClick={handleLookup}
                className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
              >
                Lookup
              </button>
            </div>
          </div>

          <div className="grid gap-4">
            <Link
              href="/bundles"
              className="p-4 border rounded-lg bg-white/5 hover:bg-white/10 transition-colors"
            >
              <h2 className="text-xl font-semibold">Live Bundles</h2>
              <p className="text-gray-600 dark:text-gray-400">
                View and monitor transaction bundles
              </p>
            </Link>
          </div>
        </div>
      </main>
    </div>
  );
}
