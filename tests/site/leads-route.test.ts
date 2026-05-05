import assert from "node:assert/strict";
import { afterEach, test } from "node:test";
import { POST } from "../../app/api/leads/route";

const originalFetch = globalThis.fetch;

const validLead = {
  name: "Nyasha Hama",
  email: "nyasha@example.com",
  company: "Guard Rail",
  role: "Founder",
  routeCount: "2",
  useCase: "Pilot the webhook route.",
};

afterEach(() => {
  delete process.env.LEAD_WEBHOOK_URL;
  delete process.env.LEAD_WEBHOOK_TOKEN;
  globalThis.fetch = originalFetch;
});

function leadRequest(body: unknown, raw = false): Request {
  return new Request("http://localhost/api/leads", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: raw ? String(body) : JSON.stringify(body),
  });
}

async function responseJson(response: Response): Promise<unknown> {
  return response.json();
}

test("rejects malformed JSON", async () => {
  const response = await POST(leadRequest("{", true));

  assert.equal(response.status, 400);
  assert.deepEqual(await responseJson(response), { error: "invalid_json" });
});

test("rejects non-object lead payloads", async () => {
  const response = await POST(leadRequest(null));

  assert.equal(response.status, 400);
  assert.deepEqual(await responseJson(response), { error: "invalid_lead" });
});

test("returns 503 when no lead sink is configured", async () => {
  const response = await POST(leadRequest(validLead));

  assert.equal(response.status, 503);
  assert.deepEqual(await responseJson(response), { error: "lead_sink_not_configured" });
});

test("returns 502 when lead sink rejects the submission", async () => {
  process.env.LEAD_WEBHOOK_URL = "https://leads.example.test/webhook";
  globalThis.fetch = async () => new Response("upstream error", { status: 500 });

  const response = await POST(leadRequest(validLead));

  assert.equal(response.status, 502);
  assert.deepEqual(await responseJson(response), { error: "lead_sink_failed" });
});

test("forwards valid leads with the configured webhook token", async () => {
  const calls: Array<{ input: Parameters<typeof fetch>[0]; init?: Parameters<typeof fetch>[1] }> = [];
  process.env.LEAD_WEBHOOK_URL = "https://leads.example.test/webhook";
  process.env.LEAD_WEBHOOK_TOKEN = "pilot-token";
  globalThis.fetch = async (input, init) => {
    calls.push({ input, init });
    return new Response(null, { status: 204 });
  };

  const response = await POST(leadRequest({ ...validLead, name: "  Nyasha Hama  " }));

  assert.equal(response.status, 202);
  assert.deepEqual(await responseJson(response), { ok: true });
  assert.equal(calls.length, 1);
  assert.equal(calls[0].input, "https://leads.example.test/webhook");
  assert.equal(calls[0].init?.method, "POST");
  assert.equal(calls[0].init?.cache, "no-store");
  assert.equal(
    (calls[0].init?.headers as Record<string, string>)["x-guardrail-lead-secret"],
    "pilot-token",
  );

  const forwardedBody = JSON.parse(String(calls[0].init?.body)) as Record<string, unknown>;
  assert.equal(forwardedBody.name, "Nyasha Hama");
  assert.equal(forwardedBody.email, validLead.email);
  assert.equal(forwardedBody.source, "guard-rail-site");
  assert.match(String(forwardedBody.submittedAt), /^\d{4}-\d{2}-\d{2}T/);
});
