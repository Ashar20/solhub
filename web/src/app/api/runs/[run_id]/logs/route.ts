import { NextRequest, NextResponse } from "next/server";

const BACKEND_BASE =
  process.env.NEXT_PUBLIC_API_BASE_URL ?? "http://localhost:8080";

export async function GET(
  req: NextRequest,
  { params }: { params: { run_id: string } },
) {
  const { run_id } = params;
  const token = req.nextUrl.searchParams.get("token");

  if (!token) {
    return new NextResponse("missing token", { status: 401 });
  }

  const upstream = await fetch(
    `${BACKEND_BASE}/v1/runs/${encodeURIComponent(run_id)}/logs`,
    {
      headers: {
        Authorization: `Bearer ${token}`,
        Accept: "text/event-stream",
      },
      cache: "no-store",
    },
  );

  if (!upstream.ok || !upstream.body) {
    const body = await upstream.text().catch(() => "");
    return new NextResponse(body || "upstream error", {
      status: upstream.status,
    });
  }

  return new NextResponse(upstream.body, {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream",
      "Cache-Control": "no-cache, no-transform",
      Connection: "keep-alive",
      "X-Accel-Buffering": "no",
    },
  });
}
