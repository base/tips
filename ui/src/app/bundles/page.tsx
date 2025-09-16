import Link from "next/link";
import { db } from "../../db";
import { bundles } from "../../db/schema";

export default async function BundlesPage() {
  const allBundles = await db.select().from(bundles);
  return (
    <div>
      <main className="flex flex-col gap-[32px] row-start-2 items-center sm:items-start">
        <div className="w-full max-w-4xl">
          <h2 className="text-xl font-semibold mb-4">
            Transaction Bundles ({allBundles.length})
          </h2>
          {allBundles.length > 0 ? (
            <div className="grid gap-4">
              {allBundles.map((bundle) => (
                <div
                  key={bundle.id}
                  className="p-4 border rounded-lg bg-white/5"
                >
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="font-medium">Bundle {bundle.id}</h3>
                    <Link
                      href={`/bundle/${bundle.id}`}
                      className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                    >
                      View Details
                    </Link>
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400 space-y-1">
                    <p>Block: {bundle.blockNumber}</p>
                    <p>Transactions: {bundle.txs?.length || 0}</p>
                    <p>Senders: {bundle.senders?.length || 0}</p>
                    <p>Min Base Fee: {bundle.minimumBaseFee}</p>
                  </div>
                  <p className="text-xs text-gray-500 mt-2">
                    Created: {new Date(bundle.createdAt).toLocaleDateString()}
                  </p>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-600 dark:text-gray-400">
              No bundles found in database.
            </p>
          )}
        </div>
      </main>
    </div>
  );
}
