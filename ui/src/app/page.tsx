"use client";

import { useRouter } from "next/navigation";

export default function Home() {
  const _router = useRouter();

  return (
    <div>
      <main className="flex flex-col gap-[32px] row-start-2 items-center sm:items-start">
        <div className="w-full max-w-4xl">
          <h1 className="text-2xl font-bold mb-8">
            Tips - Transaction Inclusion Pipeline Services
          </h1>
        </div>
      </main>
    </div>
  );
}
