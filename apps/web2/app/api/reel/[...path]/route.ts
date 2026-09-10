import { NextRequest, NextResponse } from "next/server";
import { reelProxyPathAllowed } from "../../../catalogue";

const apiBaseUrl = process.env.REEL_API_URL ?? "http://localhost:3000";

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ path: string[] }> },
) {
  const { path } = await params;
  const reelPath = path.join("/");
  if (!reelProxyPathAllowed(reelPath)) {
    return NextResponse.json(
      { error: { code: "not_found", message: "Unknown catalogue route" } },
      { status: 404 },
    );
  }
  const upstream = new URL(reelPath, `${apiBaseUrl.replace(/\/$/, "")}/`);
  for (const key of ["language", "cursor", "ids", "view", "include"]) {
    const value = request.nextUrl.searchParams.get(key);
    if (value) upstream.searchParams.set(key, value);
  }

  try {
    const response = await fetch(upstream, {
      cache: "no-store",
      headers: { accept: "application/json" },
      signal: AbortSignal.any([request.signal, AbortSignal.timeout(15_000)]),
    });
    return new NextResponse(await response.text(), {
      status: response.status,
      headers: {
        "content-type":
          response.headers.get("content-type") ?? "application/json",
      },
    });
  } catch {
    return NextResponse.json(
      {
        error: {
          code: "api_unavailable",
          message: "The Reel API could not be reached",
        },
      },
      { status: 502 },
    );
  }
}
