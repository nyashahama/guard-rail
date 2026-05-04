import { NextResponse } from "next/server";

type LeadPayload = {
  name?: unknown;
  email?: unknown;
  company?: unknown;
  role?: unknown;
  routeCount?: unknown;
  useCase?: unknown;
};

const MAX_TEXT = 1200;

function cleanString(value: unknown, maxLength: number): string | null {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  if (!trimmed || trimmed.length > maxLength) return null;
  return trimmed;
}

function isEmail(value: string): boolean {
  return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value);
}

export async function POST(request: Request) {
  let body: LeadPayload;

  try {
    body = (await request.json()) as LeadPayload;
  } catch {
    return NextResponse.json({ error: "invalid_json" }, { status: 400 });
  }

  const name = cleanString(body.name, 120);
  const email = cleanString(body.email, 180);
  const company = cleanString(body.company, 180);
  const role = cleanString(body.role, 180);
  const routeCount = cleanString(body.routeCount, 80);
  const useCase = cleanString(body.useCase, MAX_TEXT);

  if (!name || !email || !company || !role || !routeCount || !useCase || !isEmail(email)) {
    return NextResponse.json({ error: "invalid_lead" }, { status: 400 });
  }

  const webhookUrl = process.env.LEAD_WEBHOOK_URL;
  if (!webhookUrl) {
    return NextResponse.json({ error: "lead_sink_not_configured" }, { status: 503 });
  }

  const headers: HeadersInit = {
    "content-type": "application/json",
  };

  if (process.env.LEAD_WEBHOOK_SECRET) {
    headers["x-guardrail-lead-secret"] = process.env.LEAD_WEBHOOK_SECRET;
  }

  let response: Response;

  try {
    response = await fetch(webhookUrl, {
      method: "POST",
      headers,
      body: JSON.stringify({
        name,
        email,
        company,
        role,
        routeCount,
        useCase,
        source: "guard-rail-site",
        submittedAt: new Date().toISOString(),
      }),
      cache: "no-store",
    });
  } catch {
    return NextResponse.json({ error: "lead_sink_failed" }, { status: 502 });
  }

  if (!response.ok) {
    return NextResponse.json({ error: "lead_sink_failed" }, { status: 502 });
  }

  return NextResponse.json({ ok: true }, { status: 202 });
}
