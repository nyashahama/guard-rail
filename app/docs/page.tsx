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

      <div className="relative z-[1] max-w-[1100px] mx-auto px-6 sm:px-8 lg:px-12 py-[110px]">
        <p className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan mb-4">
          {"// Pilot onboarding docs"}
        </p>
        <h1 className="font-extrabold tracking-[-0.025em] leading-[1.03] text-[44px] sm:text-[64px] md:text-[72px] max-w-[18ch] mb-8">
          Guard Rail Pilot Runbook
        </h1>
        <p className="font-mono text-[14px] text-white/55 max-w-[70ch] leading-[1.65] mb-16">
          A practical, deployment-ready path for teams running a policy runtime in front
          of internal APIs, Zapier, Make, or custom webhook automation.
        </p>

        <section
          id="pilot-setup"
          className="mb-14 border border-white/15 p-8 sm:p-10 bg-surface"
        >
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">
            Pilot Setup
          </h2>
          <p className="font-mono text-[13px] text-white/65 mb-6 leading-[1.65]">
            Start with the container flow and route/policy files. The checklist below is
            the minimum viable sequence for proof-of-value.
          </p>
          <ul className="font-mono text-[12px] text-white/70 list-none space-y-2">
            <li>1) Prepare routes and policies in mounted config volume.</li>
            <li>2) Configure Postgres, admin token, tenant binding, and persistence mode.</li>
            <li>3) Run migrations and validate `/health` and `/ready`.</li>
            <li>4) Create tenant, tenant key, and bind route list.</li>
            <li>5) Execute allow/deny smoke calls and confirm audit lookup.</li>
          </ul>
          <div className="mt-7">
            <a
              href="#quickstart"
              className="font-mono text-[11px] text-black bg-white px-4 py-2 no-underline inline-flex"
            >
              Open quickstart
            </a>
          </div>
        </section>

        <section
          id="policy-integration"
          className="mb-14 border border-white/15 p-8 sm:p-10 bg-surface"
        >
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">
            Policy + Integration
          </h2>
          <p className="font-mono text-[13px] text-white/65 mb-6 leading-[1.65]">
            The route file and policy file model is the control plane for this pilot.
            Start with one policy per route for predictable behavior.
          </p>
          <div className="grid gap-4 sm:grid-cols-2">
            <a
              href="#policy-cookbook"
              className="font-mono text-[11px] border border-white/15 px-4 py-3 text-white/70 hover:text-white transition-colors"
            >
              Policy cookbook
            </a>
            <a
              href="#webhooks-guide"
              className="font-mono text-[11px] border border-white/15 px-4 py-3 text-white/70 hover:text-white transition-colors"
            >
              Webhook integration guide
            </a>
          </div>
        </section>

        <section
          id="integration"
          className="mb-14 border border-white/15 p-8 sm:p-10 bg-surface"
        >
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">
            API Reference
          </h2>
          <p className="font-mono text-[13px] text-white/65 mb-6 leading-[1.65]">
            Execution, audit, replay, and admin routes are documented for pilot ops.
            Use these paths to build your first policy-backed endpoint.
          </p>
          <a
            href="#api-reference"
            className="font-mono text-[11px] border border-white/15 px-4 py-3 text-white/70 hover:text-white transition-colors inline-block"
          >
            API reference
          </a>
        </section>

        <section className="grid gap-4 sm:grid-cols-2">
          <a
            href="#docker-pilot-guide"
            className="border border-white/15 p-8 bg-surface block text-white/70 hover:text-white hover:border-white/35"
          >
            <h3 className="font-mono text-[16px] font-bold text-white mb-4 tracking-[-0.01em]">
              Docker Pilot Guide
            </h3>
            <p className="font-mono text-[12px] text-white/60 leading-[1.6]">
              Container command examples and startup sequence for the supported deployment path.
            </p>
          </a>
          <a
            href="#scripted-demo"
            className="border border-white/15 p-8 bg-surface block text-white/70 hover:text-white hover:border-white/35"
          >
            <h3 className="font-mono text-[16px] font-bold text-white mb-4 tracking-[-0.01em]">
              Scripted Demo
            </h3>
            <p className="font-mono text-[12px] text-white/60 leading-[1.6]">
              5-minute scripted flow: allowed request, blocked callback, replay, and audit lookup.
            </p>
          </a>
        </section>

        <section id="quickstart" className="mt-14 border border-white/15 p-8 sm:p-10 bg-surface">
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">Quickstart</h2>
          <p className="font-mono text-[13px] text-white/65 leading-[1.65]">
            See the canonical onboarding commands in the repository docs for endpoint checks,
            tenant setup, and first allowed/blocked execution paths.
          </p>
        </section>

        <section id="policy-cookbook" className="mt-14 border border-white/15 p-8 sm:p-10 bg-surface">
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">Policy Cookbook</h2>
          <p className="font-mono text-[13px] text-white/65 leading-[1.65]">
            Use one policy family per route first. Start with callback allowlist, then add size checks,
            then sensitive-field checks.
          </p>
        </section>

        <section id="webhooks-guide" className="mt-14 border border-white/15 p-8 sm:p-10 bg-surface">
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">Webhook Guide</h2>
          <p className="font-mono text-[13px] text-white/65 leading-[1.65]">
            Zapier, Make, and custom webhooks all call the same execution endpoint.
            Keep one route contract and map callbacks to explicit fields.
          </p>
        </section>

        <section id="api-reference" className="mt-14 border border-white/15 p-8 sm:p-10 bg-surface">
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">API Reference</h2>
          <p className="font-mono text-[13px] text-white/65 leading-[1.65]">
            The endpoint coverage includes health/readiness, execute, audit, replay, and admin operations.
          </p>
        </section>

        <section
          id="docker-pilot-guide"
          className="mt-14 border border-white/15 p-8 sm:p-10 bg-surface"
        >
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">Docker Pilot Guide</h2>
          <p className="font-mono text-[13px] text-white/65 leading-[1.65]">
            Container startup includes migration, readiness check, and route/policy file reload behavior.
          </p>
        </section>

        <section id="scripted-demo" className="mt-14 border border-white/15 p-8 sm:p-10 bg-surface">
          <h2 className="font-extrabold text-[30px] tracking-[-0.02em] mb-6">Scripted Demo</h2>
          <p className="font-mono text-[13px] text-white/65 leading-[1.65]">
            Use the demo script template for a first real pilot walkthrough with audit and replay evidence.
          </p>
        </section>
      </div>
    </main>
  );
}
