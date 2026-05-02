import { randomUUID } from "node:crypto";
import { NextRequest, NextResponse } from "next/server";

type LeadPayload = {
  name: string;
  company: string;
  email: string;
  use_case: string;
  integration_type: string;
  timeline: string;
};

const requiredLeadFields = [
  "name",
  "company",
  "email",
  "use_case",
  "integration_type",
  "timeline",
] as const;

function normalizeBody(body: unknown): LeadPayload | null {
  if (!body || typeof body !== "object") {
    return null;
  }

  const candidate = body as Record<string, unknown>;

  const normalized: LeadPayload = {
    name: typeof candidate.name === "string" ? candidate.name.trim() : "",
    company: typeof candidate.company === "string" ? candidate.company.trim() : "",
    email: typeof candidate.email === "string" ? candidate.email.trim() : "",
    use_case: typeof candidate.use_case === "string" ? candidate.use_case.trim() : "",
    integration_type:
      typeof candidate.integration_type === "string"
        ? candidate.integration_type.trim()
        : "",
    timeline: typeof candidate.timeline === "string" ? candidate.timeline.trim() : "",
  };

  return normalized;
}

export async function POST(request: NextRequest) {
  let body: unknown;

  try {
    body = await request.json();
  } catch {
    return NextResponse.json(
      { error: "Request body must be valid JSON." },
      { status: 400 }
    );
  }

  const payload = normalizeBody(body);

  if (!payload) {
    return NextResponse.json(
      { error: "Request body must be a JSON object." },
      { status: 400 }
    );
  }

  const missingFields = requiredLeadFields.filter((field) => !payload[field]);
  if (missingFields.length > 0) {
    return NextResponse.json(
      { error: `Missing required field(s): ${missingFields.join(", ")}` },
      { status: 400 }
    );
  }

  const webhookUrl = process.env.LEAD_WEBHOOK_URL || process.env.NEXT_PUBLIC_LEAD_WEBHOOK_URL;
  if (!webhookUrl) {
    return NextResponse.json(
      { error: "Lead webhook URL is not configured." },
      { status: 503 }
    );
  }

  const requestId = randomUUID();
  const leadPayload = {
    ...payload,
    request_id: requestId,
    created_at: new Date().toISOString(),
  };

  try {
    const webhookResponse = await fetch(webhookUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(leadPayload),
    });

    if (!webhookResponse.ok) {
      return NextResponse.json(
        {
          error: "Lead webhook could not accept the request.",
          request_id: requestId,
          status_code: webhookResponse.status,
        },
        { status: 502 }
      );
    }
  } catch {
    return NextResponse.json(
      { error: "Lead webhook request failed.", request_id: requestId },
      { status: 502 }
    );
  }

  return NextResponse.json({ request_id: requestId, status: "accepted" });
}
