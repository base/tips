import { redirect } from "next/navigation";
import { getTransactionMetadataByHash } from "../../../lib/s3";

interface PageProps {
  params: Promise<{ txnHash: string }>;
}

export default async function TransactionRedirectPage({ params }: PageProps) {
  const { txnHash } = await params;

  const metadata = await getTransactionMetadataByHash(txnHash);

  if (!metadata) {
    return (
      <div className="flex flex-col gap-4 p-8">
        <h1 className="text-2xl font-bold">Transaction Not Found</h1>
        <p className="text-gray-600 dark:text-gray-400">
          No transaction found with hash: {txnHash}
        </p>
      </div>
    );
  }

  redirect(`/transactions/${metadata.sender}/${metadata.nonce}`);
}
