import Link from "next/link";
import { getCanonicalTransactionLog } from "../../../../lib/s3";

interface PageProps {
  params: Promise<{ account: string; nonce: string }>;
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

export default async function TransactionHistoryPage({ params }: PageProps) {
  const { account, nonce } = await params;

  const transactionLog = await getCanonicalTransactionLog(account, nonce);

  if (!transactionLog) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <h1 className="text-2xl font-bold">Transaction Not Found</h1>
        <p className="text-gray-600 dark:text-gray-400">
          No transaction history found for account {account} with nonce {nonce}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-8">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-bold">Transaction History</h1>
        <div className="text-sm text-gray-600 dark:text-gray-400">
          <p>
            <strong>Account:</strong> {account}
          </p>
          <p>
            <strong>Nonce:</strong> {nonce}
          </p>
        </div>
      </div>

      <div className="flex flex-col gap-4">
        <h2 className="text-xl font-semibold">
          Event Timeline ({transactionLog.event_log.length} events)
        </h2>

        {transactionLog.event_log.length > 0 ? (
          <div className="space-y-4">
            {transactionLog.event_log.map((event, index) => {
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
            No events found in transaction history.
          </p>
        )}
      </div>
    </div>
  );
}
