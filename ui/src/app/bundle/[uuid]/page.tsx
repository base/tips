import { eq } from "drizzle-orm";
import Link from "next/link";
import { db } from "../../../db";
import { bundles } from "../../../db/schema";
import { getBundleTransactionHashes } from "../../../lib/s3";

interface PageProps {
  params: Promise<{ uuid: string }>;
}

async function CancelBundleButton({ bundleId }: { bundleId: string }) {
  async function cancelBundle() {
    "use server";
    console.log(`Cancelling bundle: ${bundleId}`);
  }

  return (
    <form action={cancelBundle}>
      <button
        type="submit"
        className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition-colors"
      >
        Cancel Bundle
      </button>
    </form>
  );
}

export default async function BundleDetailPage({ params }: PageProps) {
  const { uuid } = await params;

  const [bundle] = await db.select().from(bundles).where(eq(bundles.id, uuid));

  if (!bundle) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <h1 className="text-2xl font-bold">Bundle Not Found</h1>
        <p className="text-gray-600 dark:text-gray-400">
          No bundle found with ID: {uuid}
        </p>
      </div>
    );
  }

  const s3TransactionHashes = await getBundleTransactionHashes(uuid);

  return (
    <div className="flex flex-col gap-6 p-8">
      <div className="flex items-center justify-between">
        <div className="flex flex-col gap-2">
          <h1 className="text-2xl font-bold">Bundle Details</h1>
          <p className="text-sm text-gray-600 dark:text-gray-400">
            Bundle ID: {bundle.id}
          </p>
        </div>
        <CancelBundleButton bundleId={bundle.id} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="space-y-4">
          <div className="border rounded-lg p-4 bg-white/5">
            <h3 className="font-semibold mb-2">Bundle Information</h3>
            <div className="space-y-2 text-sm">
              <div>
                <strong>Block Number:</strong> {bundle.blockNumber || "Pending"}
              </div>
              <div>
                <strong>Minimum Base Fee:</strong>{" "}
                {bundle.minimumBaseFee || "N/A"}
              </div>
              <div>
                <strong>Senders:</strong> {bundle.senders?.length || 0}
              </div>
              <div>
                <strong>Total Transactions:</strong> {bundle.txs?.length || 0}
              </div>
              <div>
                <strong>Reverting Transactions:</strong>{" "}
                {bundle.revertingTxHashes?.length || 0}
              </div>
              <div>
                <strong>Dropping Transactions:</strong>{" "}
                {bundle.droppingTxHashes?.length || 0}
              </div>
            </div>
          </div>

          <div className="border rounded-lg p-4 bg-white/5">
            <h3 className="font-semibold mb-2">Timestamps</h3>
            <div className="space-y-2 text-sm">
              <div>
                <strong>Created:</strong>{" "}
                {new Date(bundle.createdAt).toLocaleString()}
              </div>
              <div>
                <strong>Updated:</strong>{" "}
                {new Date(bundle.updatedAt).toLocaleString()}
              </div>
              {bundle.minTimestamp && (
                <div>
                  <strong>Min Timestamp:</strong>{" "}
                  {new Date(bundle.minTimestamp * 1000).toLocaleString()}
                </div>
              )}
              {bundle.maxTimestamp && (
                <div>
                  <strong>Max Timestamp:</strong>{" "}
                  {new Date(bundle.maxTimestamp * 1000).toLocaleString()}
                </div>
              )}
            </div>
          </div>
        </div>

        <div className="space-y-4">
          {bundle.senders && bundle.senders.length > 0 && (
            <div className="border rounded-lg p-4 bg-white/5">
              <h3 className="font-semibold mb-2">
                Senders ({bundle.senders.length})
              </h3>
              <div className="space-y-1 text-sm max-h-48 overflow-y-auto">
                {bundle.senders.map((sender) => (
                  <div key={sender} className="font-mono text-xs break-all">
                    {sender}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="space-y-4">
        <h2 className="text-xl font-semibold">
          Transaction Hashes ({bundle.txnHashes?.length || 0})
        </h2>

        {bundle.txnHashes && bundle.txnHashes.length > 0 ? (
          <div className="grid gap-2">
            {bundle.txnHashes.map((hash, index) => (
              <div
                key={hash}
                className="flex items-center justify-between p-3 border rounded-lg bg-white/5"
              >
                <div className="flex flex-col gap-1">
                  <span className="font-mono text-sm">{hash}</span>
                  <span className="text-xs text-gray-500">
                    Transaction #{index + 1}
                  </span>
                </div>
                <Link
                  href={`/transaction/${hash}`}
                  className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                >
                  View Details
                </Link>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-gray-600 dark:text-gray-400">
            No transaction hashes found.
          </p>
        )}

        {s3TransactionHashes && s3TransactionHashes.length > 0 && (
          <div className="border-t pt-4">
            <h3 className="font-semibold mb-2">
              S3 Transaction Hashes ({s3TransactionHashes.length})
            </h3>
            <div className="grid gap-2">
              {s3TransactionHashes.map((hash, index) => (
                <div
                  key={hash}
                  className="flex items-center justify-between p-3 border rounded-lg bg-white/5"
                >
                  <div className="flex flex-col gap-1">
                    <span className="font-mono text-sm">{hash}</span>
                    <span className="text-xs text-gray-500">
                      S3 Transaction #{index + 1}
                    </span>
                  </div>
                  <Link
                    href={`/transaction/${hash}`}
                    className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                  >
                    View Details
                  </Link>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
