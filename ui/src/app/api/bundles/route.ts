import { NextResponse } from "next/server";
import { listAllBundleKeys } from "@/lib/s3";

export async function GET() {
  try {
    const bundleKeys = await listAllBundleKeys();
    return NextResponse.json(bundleKeys);
  } catch (error) {
    console.error("Error fetching bundles:", error);
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 },
    );
  }
}
