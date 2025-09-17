import { NextResponse } from "next/server";
import { db } from "@/db";
import { bundles } from "@/db/schema";

export interface Bundle {
  id: string;
  txnHashes: string[] | null;
}

export async function GET() {
  try {
    const allBundles = await db
      .select({
        id: bundles.id,
        txnHashes: bundles.txnHashes,
      })
      .from(bundles);

    return NextResponse.json(allBundles);
  } catch (error) {
    console.error("Error fetching bundles:", error);
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 },
    );
  }
}
