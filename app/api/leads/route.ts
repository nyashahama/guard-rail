import { randomUUID } from "node:crypto";
import { appendFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { NextRequest, NextResponse } from "next/server";

type LeadStoreResult = {
  saved: boolean;
  path?: string;
};

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

type LeadPayloadWithMeta = LeadPayload & {
  request_id: string;
  created_at: string;
};

const maxFieldLength = 2000;
const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

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
      typeof candidate.integration_type === "string" ? candidate.integration_type.trim() : "",
    timeline: typeof candidate.timeline === "string" ? candidate.timeline.trim() : "",
  };

  return normalized;
}

function validateLeadPayload(payload: LeadPayload): string[] {
  const errors: string[] = [];

  if (!emailRegex.test(payload.email)) {
    errors.push("email must be a valid email address");
  }

  requiredLeadFields.forEach((field) => {
    const value = payload[field];
    if (value.length > maxFieldLength) {
      errors.push(`${field} exceeds maximum length of ${maxFieldLength} characters`);
    }
  });

  return errors;
}

function getFallbackLeadPath() {
  const fallbackPath =
    process.env.LEAD_FALLBACK_PATH || path.join(process.cwd(), "tmp", "leads.ndjson");

  return path.resolve(fallbackPath);
}

async function persistLeadLocally(payload: LeadPayloadWithMeta): Promise<LeadStoreResult> {
  const fallbackPath = getFallbackLeadPath();
  const dir = path.dirname(fallbackPath);

  try {
    await mkdir(dir, { recursive: true });
    await appendFile(fallbackPath, `${JSON.stringify(payload)}\n`, "utf8");
    return { saved: true, path: fallbackPath };
  } catch {
    return { saved: false };
  }
}

function getWebhookUrl(): string | undefined {
  return process.env.LEAD_WEBHOOK_URL || process.env.NEXT_PUBLIC_LEAD_WEBHOOK_URL;
}

export async function POST(request: NextRequest) {
  let body: unknown;

  try {
    body = await request.json();
  } catch {
    return NextResponse.json(
      { error: "Request body must be a JSON object with valid JSON values." },
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

  const validationErrors = validateLeadPayload(payload);
  if (validationErrors.length > 0) {
    return NextResponse.json(
      { error: `Invalid field(s): ${validationErrors.join(", ")}` },
      { status: 400 }
    );
  }

  const requestId = randomUUID();
  const leadPayload: LeadPayloadWithMeta = {
    ...payload,
    request_id: requestId,
    created_at: new Date().toISOString(),
  };

  const webhookUrl = getWebhookUrl();
  if (!webhookUrl) {
    const leadResult = await persistLeadLocally(leadPayload);

    if (!leadResult.saved) {
      return NextResponse.json(
        { error: "Lead webhook URL is not configured and local lead capture is unavailable." },
        { status: 503 }
      );
    }

    return NextResponse.json({
      status: "accepted",
      request_id: requestId,
      warning:
        "Lead webhook URL is not configured. This lead was captured locally and will be reviewed by the onboarding team.",
      delivered_via: "fallback",
      fallback_path: leadResult.path,
    });
  }

  try {
    const webhookResponse = await fetch(webhookUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(leadPayload),
    });

    if (!webhookResponse.ok) {
      const fallbackLeadResult = await persistLeadLocally(leadPayload);
      if (fallbackLeadResult.saved) {
        return NextResponse.json(
          {
            status: "accepted",
            request_id: requestId,
            warning:
              "Lead webhook could not accept the request. It was captured locally for manual follow-up.",
            delivered_via: "fallback",
            fallback_path: fallbackLeadResult.path,
          },
          { status: 202 }
        );
      }

      return NextResponse.json(
        {
          error: `Lead webhook could not accept the request (status ${webhookResponse.status}) and local fallback storage is unavailable.`,
        },
        { status: 502 }
      );
    }
  } catch {
    const fallbackLeadResult = await persistLeadLocally(leadPayload);

    if (fallbackLeadResult.saved) {
      return NextResponse.json(
        {
          status: "accepted",
          request_id: requestId,
          warning:
            "Lead webhook request failed. It was captured locally and will be reviewed by the onboarding team.",
          delivered_via: "fallback",
          fallback_path: fallbackLeadResult.path,
        },
        { status: 202 }
      );
    }

    return NextResponse.json(
      { error: "Lead webhook request failed and local fallback storage is unavailable." },
      { status: 502 }
    );
  }

  return NextResponse.json({
    status: "accepted",
    request_id: requestId,
    delivered_via: "webhook",
  });
}
