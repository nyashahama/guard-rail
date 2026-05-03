import path from "node:path";

type DocsFileKind = "docs" | "policy";

export type DocsManifestItem = {
  slug: string;
  title: string;
  description: string;
  file: string;
  kind: DocsFileKind;
};

const DOC_ROOT = path.join(
  process.cwd(),
  "guard-rail-engine",
  "deploy",
  "onboarding"
);

export const docsManifest: DocsManifestItem[] = [
  {
    slug: "overview",
    title: "Pilot Onboarding",
    description: "Start here for the current runtime-only onboarding path and scope.",
    file: "README.md",
    kind: "docs",
  },
  {
    slug: "quickstart",
    title: "Quickstart",
    description: "Bring a protected endpoint live with one route and tenant setup.",
    file: "quickstart.md",
    kind: "docs",
  },
  {
    slug: "api-reference",
    title: "API Reference",
    description: "Execution, audit, replay, and admin endpoint behavior for pilots.",
    file: "api-reference.md",
    kind: "docs",
  },
  {
    slug: "policy-cookbook",
    title: "Policy Cookbook",
    description: "Practical policy templates and examples for the first hardening pass.",
    file: "policy-cookbook.md",
    kind: "docs",
  },
  {
    slug: "docker-pilot-guide",
    title: "Docker Pilot Guide",
    description: "Container deployment setup, readiness checks, and rollout order.",
    file: "docker-pilot-guide.md",
    kind: "docs",
  },
  {
    slug: "webhooks-guide",
    title: "Webhook Integration Guide",
    description: "Zapier, Make, and custom webhook setup for `/v1/execute/{route}`.",
    file: "webhooks-guide.md",
    kind: "docs",
  },
  {
    slug: "scripted-demo",
    title: "Scripted Demo",
    description: "5-minute runbook: allowed call, blocked call, audit lookup, replay.",
    file: "scripted-demo.md",
    kind: "docs",
  },
  {
    slug: "callback-allowlist",
    title: "Sample Policy: callback-allowlist",
    description: "Policy template that blocks unapproved callback URL domains.",
    file: path.join("policies", "callback-allowlist.yaml"),
    kind: "policy",
  },
  {
    slug: "sa-id-pii-block",
    title: "Sample Policy: sa-id-pii-block",
    description: "Policy template to block SA ID / PII-like patterns.",
    file: path.join("policies", "sa-id-pii-block.yaml"),
    kind: "policy",
  },
  {
    slug: "payload-size-limit",
    title: "Sample Policy: payload-size-limit",
    description: "Policy template to enforce max payload bytes before forwarding.",
    file: path.join("policies", "payload-size-limit.yaml"),
    kind: "policy",
  },
];

export const docsChecklist = [
  "Copy supported deployment files into a pilot config volume.",
  "Set strong admin token and separate admin listener binding.",
  "Run `migrate` and verify `/ready` before any traffic.",
  "Create tenant, tenant API key, and tenant-route binding.",
  "Run one allowed and one blocked request.",
  "Confirm audit evidence for both outcomes and replay behavior.",
];

export function resolveDocFile(slug: string) {
  const item = docsManifest.find((doc) => doc.slug === slug);
  if (!item) return undefined;

  return path.join(DOC_ROOT, item.file);
}

export function resolveDocBySlug(slug: string) {
  return docsManifest.find((doc) => doc.slug === slug);
}
