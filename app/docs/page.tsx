import Link from "next/link";
import { docsChecklist, docsManifest } from "./_catalog";

const docsByCategory = {
  docs: docsManifest.filter((item) => item.kind === "docs"),
  policies: docsManifest.filter((item) => item.kind === "policy"),
};

function DocsSection({
  title,
  description,
  items,
}: {
  title: string;
  description: string;
  items: typeof docsManifest;
}) {
  return (
    <section className="mt-8 border border-white/15 bg-surface px-6 py-8 sm:px-10 sm:py-10">
      <div className="mb-5 flex flex-col gap-1.5">
        <h2 className="font-extrabold tracking-[-0.02em] leading-none text-[26px] sm:text-[34px]">
          {title}
        </h2>
        <p className="font-mono text-[12px] text-white/60">{description}</p>
      </div>
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
        {items.map((doc) => (
          <Link
            href={`/docs/${doc.slug}`}
            key={doc.slug}
            className="border border-white/10 p-5 transition-colors duration-200 hover:border-cyan/50 hover:bg-[#10101a]"
          >
            <p className="font-mono text-[10px] uppercase tracking-[0.16em] text-cyan mb-3">
              {doc.kind}
            </p>
            <h3 className="font-bold tracking-[-0.015em] text-[22px] mb-3">{doc.title}</h3>
            <p className="font-mono text-[12px] text-white/55 leading-[1.6]">{doc.description}</p>
          </Link>
        ))}
      </div>
    </section>
  );
}

export default function DocsPage() {
  return (
    <main className="relative z-[1]">
      <div className="fixed inset-0 pointer-events-none z-0">
        <div className="bg-grid-pattern" />
        <div
          className="absolute inset-0"
          style={{
            background:
              "radial-gradient(circle at 50% 40%, rgba(0,240,255,0.15) 0%, transparent 60%)",
          }}
        />
      </div>

      <div className="relative z-[1] max-w-[1200px] mx-auto px-6 sm:px-8 lg:px-12 py-[90px]">
        <p className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan mb-4">
          {"// Pilot onboarding"}
        </p>
        <h1 className="font-extrabold tracking-[-0.025em] leading-[1.03] text-[44px] sm:text-[64px] md:text-[72px] max-w-[18ch] mb-6">
          Guard Rail Onboarding
        </h1>
        <p className="font-mono text-[14px] text-white/55 max-w-[70ch] leading-[1.7] mb-12">
          Everything you need for a design-partner pilot: documented install sequence, policy
          templates, API/API-key flow, and replay + audit checks for evidence.
        </p>

        <section className="border border-white/15 bg-surface px-6 py-8 sm:px-10 sm:py-10">
          <p className="font-mono text-[11px] tracking-[0.18em] uppercase text-cyan">Start Sequence</p>
          <h2 className="mt-2 font-extrabold tracking-[-0.02em] leading-none text-[28px] sm:text-[34px] mb-5">
            Installation checklist
          </h2>
          <ul className="mt-2 list-disc pl-5 font-mono text-[12px] text-white/70 space-y-2.5">
            {docsChecklist.map((step) => (
              <li key={step} className="leading-[1.7]">
                {step}
              </li>
            ))}
          </ul>
          <div className="mt-8 flex flex-wrap gap-3">
            <Link
              href="/docs/quickstart"
              className="font-mono text-[11px] tracking-[0.12em] uppercase px-4 py-3 border border-white/25 inline-flex transition-colors hover:border-cyan/60 hover:text-cyan"
            >
              Open Quickstart
            </Link>
            <Link
              href="/docs/webhooks-guide"
              className="font-mono text-[11px] tracking-[0.12em] uppercase px-4 py-3 border border-white/25 inline-flex transition-colors hover:border-cyan/60 hover:text-cyan"
            >
              Webhook Guide
            </Link>
            <Link
              href="/#pilot-lead-form"
              className="font-mono text-[11px] tracking-[0.12em] uppercase px-4 py-3 border border-white/25 inline-flex transition-colors hover:border-cyan/60 hover:text-cyan"
            >
              Request Onboarding Session
            </Link>
          </div>
        </section>

        <DocsSection
          title="Pilot docs"
          description="Pick the exact onboarding path needed for your first validation route."
          items={docsByCategory.docs}
        />
        <DocsSection
          title="Sample policy templates"
          description="Use these templates as starting point files in your policies directory."
          items={docsByCategory.policies}
        />
      </div>
    </main>
  );
}
