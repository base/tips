import Link from "next/link";

export default function Home() {
  return (
    <div>
      <main className="flex flex-col gap-[32px] row-start-2 items-center sm:items-start">
        <div className="w-full max-w-4xl">
          <h1 className="text-2xl font-bold mb-8">
            Tips - Transaction Inclusion Pipeline Services
          </h1>
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
