"use client";

import { useEffect, useRef } from "react";

export default function Home() {
  const containerRef = useRef<HTMLDivElement>(null);
  const indicatorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const container = containerRef.current;
    const indicator = indicatorRef.current;
    if (!container || !indicator) return;

    const handleScroll = () => {
      const scrollTop = container.scrollTop;
      const scrollHeight = container.scrollHeight - container.clientHeight;
      const pct = (scrollTop / scrollHeight) * 100;
      indicator.style.height = `${Math.max(8.33, pct)}%`;
    };

    container.addEventListener("scroll", handleScroll);
    return () => container.removeEventListener("scroll", handleScroll);
  }, []);

  return (
    <>
      {/* Ambient Background */}
      <div className="fixed inset-0 pointer-events-none z-0">
        <div className="absolute inset-0 bg-grid-pattern bg-grid opacity-20" />
        <div className="absolute inset-0 bg-radial-glow" />
        <div className="scan-effect absolute inset-0 mix-blend-screen" />
      </div>

      {/* Fixed Global Navigation */}
      <nav className="fixed top-0 left-0 right-0 z-50 flex items-center justify-between px-8 py-6 mix-blend-difference">
        <div className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-cyan flex items-center justify-center rounded-sm">
            <div className="w-2 h-2 bg-cyan" />
          </div>
          <span className="font-bold tracking-widest uppercase text-sm">
            Guard Rail
          </span>
        </div>
        <div className="flex items-center gap-6">
          <span className="font-mono text-xs text-white/50 hidden md:block">
            CONFIDENTIAL // SEED DECK v2.4
          </span>
          <a
            href="#slide-13"
            className="text-xs font-mono font-bold uppercase tracking-wider border border-white/20 px-4 py-2 hover:bg-white hover:text-black transition-colors duration-300"
          >
            Partner with us
          </a>
        </div>
      </nav>

      {/* Fixed Slide Indicator */}
      <aside className="fixed left-8 bottom-8 top-8 z-40 hidden lg:flex flex-col justify-center gap-4 mix-blend-difference">
        <div className="w-[1px] h-32 bg-white/20 relative">
          <div
            ref={indicatorRef}
            className="w-full bg-cyan absolute top-0"
            style={{ height: "8.33%" }}
          />
        </div>
        <span className="font-mono text-xs text-white/50 rotate-[-90deg] origin-left translate-x-4 w-12">
          SCROLL
        </span>
      </aside>

      <div className="fixed right-8 bottom-8 hidden md:block z-50">
        <span className="font-mono text-[10px] text-white/30 tracking-widest uppercase">
          ZAF / Enterprise Auth
        </span>
      </div>

      {/* Main Deck Container */}
      <main ref={containerRef} className="deck-container relative z-10 w-full">
        {/* SLIDE 1: HERO */}
        <section id="slide-1" className="slide pt-20">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
              <div>
                <div className="inline-flex items-center gap-2 px-3 py-1 border border-cyan/30 bg-cyan/5 rounded-full mb-8">
                  <div className="w-1.5 h-1.5 bg-cyan rounded-full animate-pulse" />
                  <span className="font-mono text-[10px] uppercase text-cyan tracking-wider">
                    Securing the next billion transactions
                  </span>
                </div>
                <h1 className="text-6xl lg:text-8xl font-bold tracking-tighter leading-[0.9] mb-6 text-glow">
                  Secure <br />
                  <span className="text-white/40">execution for</span>
                  <br />
                  modern <br />
                  automation.
                </h1>
                <p className="text-xl text-white/60 max-w-lg mb-10 font-light">
                  The enterprise-grade runtime engine. Sandboxing partner
                  integrations, workflows, and AI agents for South Africa&apos;s
                  critical financial infrastructure.
                </p>
                <div className="flex items-center gap-4 font-mono text-sm">
                  <span className="text-white/40">Initialize deck</span>
                  <svg
                    className="w-4 h-4 text-cyan animate-bounce"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M19 14l-7 7m0 0l-7-7m7 7V3"
                    />
                  </svg>
                </div>
              </div>

              <div className="relative h-[600px] hidden lg:flex items-center justify-center">
                <div className="absolute w-96 h-96 border border-white/10 rounded-full flex items-center justify-center animate-spin [animation-duration:60s]">
                  <div className="w-80 h-80 border border-cyan/20 rounded-full border-dashed" />
                  <div className="absolute top-0 w-4 h-4 bg-cyan rounded-full shadow-[0_0_20px_#00F0FF]" />
                </div>
                <div className="w-48 h-48 bg-cyan/10 rounded-full blur-3xl absolute" />
                <div className="relative w-32 h-32 glass-panel pulse-element flex items-center justify-center rounded-2xl z-10">
                  <ShieldCheckIcon />
                </div>

                <div className="absolute top-20 right-20 glass-panel px-4 py-2 flex items-center gap-2 rounded text-xs font-mono">
                  <CheckCircleIcon className="text-green" /> Payload Signed
                </div>
                <div className="absolute bottom-32 left-10 glass-panel px-4 py-2 flex items-center gap-2 rounded text-xs font-mono">
                  <ActivityIcon className="text-cyan" /> 0.04ms Latency
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 2: THE PROBLEM */}
        <section id="slide-2" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="flex flex-col lg:flex-row justify-between items-start gap-16">
              <div className="lg:w-1/2">
                <span className="font-mono text-crimson text-sm tracking-widest uppercase mb-4 block">
                  // Risk Vector
                </span>
                <h2 className="text-5xl lg:text-7xl font-bold tracking-tight mb-8">
                  Automation is scaling.
                  <br />
                  <span className="text-white/30">Control is not.</span>
                </h2>
                <p className="text-lg text-white/60 mb-6">
                  Enterprise architecture has fundamentally shifted. Zapier,
                  Make, custom scripts, and AI agents are driving operations.
                  Yet, standard perimeter security models are blind to
                  intra-workflow execution context.
                </p>
                <p className="text-lg text-white/60">
                  When an unverified third-party app executes a workflow across
                  your internal API, you don&apos;t have a firewall problem.{" "}
                  <strong className="text-white">
                    You have a runtime problem.
                  </strong>
                </p>
              </div>

              <div className="lg:w-1/2 flex flex-col gap-4">
                <div className="p-8 border border-white/5 bg-surface-light relative overflow-hidden group hover:border-crimson/50 transition-colors">
                  <div className="absolute top-0 right-0 w-16 h-16 bg-crimson/10 rounded-bl-full blur-xl" />
                  <div className="font-mono text-5xl font-bold text-crimson mb-2">
                    412%
                  </div>
                  <h3 className="text-xl font-medium mb-1">
                    Increase in API Sprawl
                  </h3>
                  <p className="text-sm text-white/40">
                    In South African banking networks over the last 36 months,
                    leading to impossible audit trails.
                  </p>
                </div>

                <div className="p-8 border border-white/5 bg-surface-light relative overflow-hidden group hover:border-crimson/50 transition-colors">
                  <div className="font-mono text-5xl font-bold text-white mb-2">
                    73%
                  </div>
                  <h3 className="text-xl font-medium mb-1">
                    Unverified 3rd Party Logic
                  </h3>
                  <p className="text-sm text-white/40">
                    Of mission-critical data transfers rely on external webhooks
                    without payload-level inspection.
                  </p>
                </div>

                <div className="p-8 border border-white/5 bg-surface-light relative overflow-hidden group hover:border-crimson/50 transition-colors flex items-center justify-between">
                  <div>
                    <h3 className="text-xl font-medium mb-1 text-crimson">
                      Shadow Workflows
                    </h3>
                    <p className="text-sm text-white/40">
                      Operations bypassing IT governance.
                    </p>
                  </div>
                  <WarningIcon className="text-crimson/50" />
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 3: THE SOLUTION */}
        <section id="slide-3" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="text-center mb-16">
              <span className="font-mono text-cyan text-sm tracking-widest uppercase mb-4 block">
                // The primitive
              </span>
              <h2 className="text-4xl lg:text-6xl font-bold tracking-tight">
                Zero-Trust Execution.
              </h2>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-4 auto-rows-[240px]">
              <div className="md:col-span-2 glass-panel p-8 rounded-xl flex flex-col justify-between group hover:border-cyan/50 transition-all duration-500 relative overflow-hidden">
                <div className="absolute -right-20 -top-20 w-64 h-64 bg-cyan/10 blur-[60px] group-hover:bg-cyan/20 transition-all" />
                <div className="relative z-10">
                  <CubeIcon className="text-cyan mb-4" />
                  <h3 className="text-2xl font-bold mb-2">Sandboxed Runtime</h3>
                  <p className="text-white/50 max-w-md">
                    eBPF-powered isolation environment. Every external call,
                    webhook, or script is executed in an ephemeral, locked-down
                    micro-VM before hitting the enterprise core.
                  </p>
                </div>
                <div className="font-mono text-xs text-white/30 truncate">
                  {"env.isolate() -> execution_context_id: GR-8922x"}
                </div>
              </div>

              <div className="glass-panel p-8 rounded-xl flex flex-col justify-between group hover:border-cyan/50 transition-all duration-500">
                <div>
                  <FileCodeIcon className="text-cyan mb-4" />
                  <h3 className="text-xl font-bold mb-2">Policy Engine</h3>
                  <p className="text-sm text-white/50">
                    Declarative YAML policies to block malicious payloads at the
                    parameter level.
                  </p>
                </div>
                <div className="w-full h-1 bg-white/10 rounded-full overflow-hidden">
                  <div className="w-3/4 h-full bg-cyan" />
                </div>
              </div>

              <div className="glass-panel p-8 rounded-xl flex flex-col justify-between group hover:border-cyan/50 transition-all duration-500">
                <div>
                  <ReplayIcon className="text-cyan mb-4" />
                  <h3 className="text-xl font-bold mb-2">
                    Deterministic Replay
                  </h3>
                  <p className="text-sm text-white/50">
                    Capture full state. If a task fails or violates policy,
                    replay the exact execution environment for debugging.
                  </p>
                </div>
              </div>

              <div className="md:col-span-2 glass-panel p-8 rounded-xl flex items-center justify-between group hover:border-cyan/50 transition-all duration-500">
                <div>
                  <DatabaseIcon className="text-cyan mb-4" />
                  <h3 className="text-2xl font-bold mb-2">
                    Cryptographic Audit Logs
                  </h3>
                  <p className="text-white/50 max-w-sm">
                    Every execution generates a tamper-proof hash. Built for
                    rigorous compliance audits.
                  </p>
                </div>
                <div className="w-64 h-full bg-void border border-white/5 rounded p-4 font-mono text-[10px] text-white/40 flex flex-col justify-end">
                  <div>[INFO] Checksum verified</div>
                  <div>[WARN] PII detected, masking...</div>
                  <div className="text-green">[PASS] Block #9924 committed</div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 4: ARCHITECTURE FLOW */}
        <section id="slide-4" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <span className="font-mono text-white/40 text-sm tracking-widest uppercase mb-12 block text-center">
              // Data Flow Topography
            </span>

            <div className="relative flex flex-col md:flex-row items-center justify-center gap-8 lg:gap-16 mt-16 w-full max-w-5xl mx-auto">
              <div className="absolute top-1/2 left-0 right-0 h-px bg-white/10 -translate-y-1/2 hidden md:block z-0" />

              <div className="z-10 group flex flex-col items-center">
                <div className="w-24 h-24 rounded-full border border-white/20 bg-surface flex items-center justify-center mb-4 group-hover:border-white transition-colors">
                  <LightningIcon className="text-white/60" />
                </div>
                <h4 className="font-bold text-center">Untrusted Trigger</h4>
                <span className="font-mono text-xs text-white/40 mt-2 text-center">
                  Webhook, Partner API,
                  <br />
                  No-Code tool
                </span>
              </div>

              <div className="z-10 hidden md:flex items-center text-cyan">
                <ArrowRightIcon />
              </div>

              <div className="z-10 flex flex-col items-center scale-110">
                <div className="w-32 h-32 rounded-2xl glass-panel pulse-element border-cyan/50 flex flex-col items-center justify-center mb-4 relative shadow-[0_0_40px_rgba(0,240,255,0.1)]">
                  <ShieldCheckIcon />
                  <span className="font-mono text-[10px] font-bold tracking-widest uppercase mt-2">
                    Guard Rail
                  </span>
                </div>
                <div className="flex gap-2 font-mono text-[10px]">
                  <span className="px-2 py-1 bg-surface-light border border-white/10 rounded">
                    Inspect
                  </span>
                  <span className="px-2 py-1 bg-surface-light border border-white/10 rounded">
                    Sanitize
                  </span>
                  <span className="px-2 py-1 bg-surface-light border border-white/10 rounded">
                    Log
                  </span>
                </div>
              </div>

              <div className="z-10 hidden md:flex items-center text-green">
                <ArrowRightIcon />
              </div>

              <div className="z-10 group flex flex-col items-center">
                <div className="w-24 h-24 rounded-lg border border-green/30 bg-green/5 flex items-center justify-center mb-4">
                  <ServerIcon className="text-green" />
                </div>
                <h4 className="font-bold text-center">Enterprise Core</h4>
                <span className="font-mono text-xs text-white/40 mt-2 text-center">
                  Internal DBs, Core Banking,
                  <br />
                  ERP Systems
                </span>
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 5: USE CASES */}
        <section id="slide-5" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="flex flex-col lg:flex-row gap-16">
              <div className="lg:w-1/3">
                <h2 className="text-4xl lg:text-5xl font-bold tracking-tight mb-6">
                  Designed for
                  <br />
                  Resilience.
                </h2>
                <p className="text-white/50 text-lg mb-8">
                  Where high-velocity automation meets zero-tolerance risk.
                </p>
              </div>

              <div className="lg:w-2/3 grid grid-cols-1 md:grid-cols-2 gap-x-8 gap-y-12">
                {[
                  {
                    title: "B2B Partner Integrations",
                    desc: "Expose APIs to logistics partners or fintechs safely. Guard Rail sits as a proxy middleware, enforcing payload shape and rate limits dynamically.",
                  },
                  {
                    title: "AI Agent Tool Use",
                    desc: "When LLMs are given access to internal tools, Guard Rail acts as the chaperone, ensuring requested actions don\u2019t violate business logic or hallucinate destructive commands.",
                  },
                  {
                    title: "Legacy Modernization",
                    desc: "Wrap vulnerable legacy SOAP or Mainframe interfaces in a modern, secure execution wrapper without rewriting the core systems.",
                  },
                  {
                    title: "Citizen Developer Gov.",
                    desc: "Allow marketing or ops teams to use Zapier/Make. Guard Rail intercepts the webhooks before they hit production databases.",
                  },
                ].map((item) => (
                  <div
                    key={item.title}
                    className="relative pl-6 border-l border-white/10 hover:border-cyan transition-colors"
                  >
                    <div className="absolute -left-[5px] top-0 w-2 h-2 rounded-full bg-cyan" />
                    <h3 className="text-xl font-bold mb-2">{item.title}</h3>
                    <p className="text-sm text-white/50">{item.desc}</p>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 6: MARKET TIMING (SA) */}
        <section
          id="slide-6"
          className="slide bg-surface-light relative border-y border-white/5"
        >
          <div className="absolute inset-0 z-0 overflow-hidden mix-blend-overlay opacity-30">
            <svg width="100%" height="100%" xmlns="http://www.w3.org/2000/svg">
              <path
                d="M 100 800 Q 300 400 600 700 T 1200 400"
                stroke="rgba(0,240,255,0.5)"
                strokeWidth="2"
                fill="none"
                className="animate-pulse"
              />
              <path
                d="M -100 600 Q 400 700 800 300 T 1400 500"
                stroke="rgba(255,255,255,0.1)"
                strokeWidth="1"
                fill="none"
              />
            </svg>
            <div className="absolute top-[40%] left-[30%] w-3 h-3 bg-cyan rounded-full map-node shadow-[0_0_15px_#00F0FF]" />
            <div className="absolute top-[60%] left-[60%] w-3 h-3 bg-cyan rounded-full map-node shadow-[0_0_15px_#00F0FF]" />
            <div className="absolute top-[30%] left-[70%] w-3 h-3 bg-cyan rounded-full map-node shadow-[0_0_15px_#00F0FF]" />
          </div>

          <div className="max-w-7xl mx-auto px-8 w-full relative z-10">
            <div className="mb-12">
              <span className="font-mono text-cyan text-sm tracking-widest uppercase mb-4 block">
                // Market Vector: Southern Africa
              </span>
              <h2 className="text-5xl lg:text-7xl font-bold tracking-tight">
                Why Context Matters.
              </h2>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
              {[
                {
                  num: "01",
                  title: "POPIA Strictures",
                  desc: "South African enterprises face stringent data residency and privacy mandates. Offshore SaaS automation routers inherently violate cross-border data transfer laws. Guard Rail deploys locally (on-prem or ZA-AWS).",
                  accent: false,
                },
                {
                  num: "02",
                  title: "Fraud Typologies",
                  desc: "High-risk financial environments require deep payload inspection. Standard API gateways check headers; Guard Rail checks the logic, halting sophisticated supply-chain injections tailored to local payment systems.",
                  accent: false,
                },
                {
                  num: "03",
                  title: "Heavy Legacy Base",
                  desc: "JSE Top 40 companies rely heavily on aging COBOL/mainframe infrastructure. Guard Rail bridges the gap, allowing rapid API-led innovation without compromising the brittle core.",
                  accent: true,
                },
              ].map((item) => (
                <div
                  key={item.num}
                  className={`bg-void p-8 border border-white/5 ${item.accent ? "border-b-2 border-b-cyan" : ""}`}
                >
                  <h4 className="font-mono text-lg font-bold mb-4 text-white/80">
                    {item.num}. {item.title}
                  </h4>
                  <p className="text-sm text-white/50 leading-relaxed">
                    {item.desc}
                  </p>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* SLIDE 7: CAPABILITIES (Tech Dense) */}
        <section id="slide-7" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="flex flex-col items-center justify-center w-full">
              <h2 className="text-3xl font-bold mb-10">System Architecture</h2>

              <div className="w-full max-w-4xl bg-black rounded-xl border border-white/10 shadow-2xl overflow-hidden">
                <div className="h-10 bg-surface border-b border-white/10 flex items-center px-4 gap-2">
                  <div className="w-3 h-3 rounded-full bg-crimson" />
                  <div className="w-3 h-3 rounded-full bg-yellow-500" />
                  <div className="w-3 h-3 rounded-full bg-green" />
                  <div className="ml-4 font-mono text-[10px] text-white/30">
                    guardrail-core-v1.4.2
                  </div>
                </div>
                <div className="p-6 font-mono text-sm leading-relaxed text-white/70 h-96 overflow-y-auto">
                  <div className="text-cyan mb-4">
                    $ ./system_check --print-capabilities
                  </div>
                  <ul className="space-y-4">
                    {[
                      {
                        status: "OK",
                        color: "text-green",
                        label: "Execution Isolation:",
                        val: "V8 Isolate Engine / WebAssembly limits compute to less than 10ms.",
                      },
                      {
                        status: "OK",
                        color: "text-green",
                        label: "Memory Bounds:",
                        val: "Strict OOM killing. Max heap allocation constrained per tenant policy.",
                      },
                      {
                        status: "OK",
                        color: "text-green",
                        label: "Network Egress:",
                        val: "Zero-trust by default. Explicit allowlisting via DNS resolution proxy.",
                      },
                      {
                        status: "OK",
                        color: "text-green",
                        label: "Secret Injection:",
                        val: "Dynamic credential fetching from HashiCorp Vault. Never stored in memory.",
                      },
                      {
                        status: "WARN",
                        color: "text-yellow-500",
                        label: "Rate Limiting:",
                        val: "Token bucket algorithm active. Throttling applied at IP + Payload hash level.",
                      },
                      {
                        status: "OK",
                        color: "text-green",
                        label: "Telemetry Output:",
                        val: "OpenTelemetry (OTLP) exporting directly to Datadog / ELK stack.",
                      },
                    ].map((row) => (
                      <li key={row.label} className="flex gap-4">
                        <span className={row.color}>[{row.status}]</span>
                        <span className="w-48 text-white">{row.label}</span>
                        <span>{row.val}</span>
                      </li>
                    ))}
                  </ul>
                  <div className="mt-6 flex items-center gap-2">
                    <span className="text-cyan">admin@guardrail:~$</span>
                    <span className="w-2 h-4 bg-white animate-pulse inline-block" />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 8: COMPETITIVE EDGE */}
        <section id="slide-8" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <h2 className="text-4xl font-bold tracking-tight mb-12">
              The Shift in Paradigm.
            </h2>

            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse min-w-[800px]">
                <thead>
                  <tr className="border-b border-white/10 text-white/50 font-mono text-sm">
                    <th className="py-6 px-4 w-1/3">Capability</th>
                    <th className="py-6 px-4 w-1/5 text-center">
                      Legacy API Gateways
                    </th>
                    <th className="py-6 px-4 w-1/5 text-center">
                      In-house Middleware
                    </th>
                    <th className="py-6 px-4 w-1/5 text-center bg-white/5 border-t border-x border-white/10 rounded-t-lg text-cyan font-bold">
                      Guard Rail
                    </th>
                  </tr>
                </thead>
                <tbody className="text-sm">
                  {[
                    {
                      cap: "Header & Token Auth",
                      legacy: true,
                      inhouse: true,
                      gr: true,
                    },
                    {
                      cap: "Deep Payload Logic Inspection",
                      legacy: false,
                      inhouse: "partial",
                      gr: true,
                    },
                    {
                      cap: "Sandboxed Execution Environment",
                      legacy: false,
                      inhouse: false,
                      gr: true,
                    },
                    {
                      cap: "Deterministic Error Replay",
                      legacy: false,
                      inhouse: false,
                      gr: true,
                    },
                    {
                      cap: "ZA Data Residency Guarantee",
                      legacy: "offshore",
                      inhouse: true,
                      gr: true,
                      last: true,
                    },
                  ].map((row) => (
                    <tr
                      key={row.cap}
                      className={`${row.last ? "" : "border-b border-white/5"} hover:bg-white/5 transition-colors`}
                    >
                      <td className="py-5 px-4 font-medium">{row.cap}</td>
                      <td className="py-5 px-4 text-center">
                        <CompareCell value={row.legacy} />
                      </td>
                      <td className="py-5 px-4 text-center">
                        <CompareCell value={row.inhouse} />
                      </td>
                      <td
                        className={`py-5 px-4 text-center bg-white/5 border-x ${row.last ? "border-b border-cyan rounded-b-lg shadow-[0_10px_20px_rgba(0,240,255,0.05)]" : "border-white/10"}`}
                      >
                        <CheckIcon className="text-cyan inline-block" />
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        </section>

        {/* SLIDE 9: BUSINESS MODEL */}
        <section id="slide-9" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="text-center mb-16">
              <h2 className="text-4xl font-bold mb-4">Value Extraction.</h2>
              <p className="text-white/50">
                Predictable compute pricing aligned to enterprise scale.
              </p>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-5xl mx-auto">
              <div className="border border-white/10 bg-surface p-8 rounded-2xl opacity-70 hover:opacity-100 transition-opacity">
                <div className="font-mono text-xs text-white/40 mb-4 block uppercase">
                  Proof of Value
                </div>
                <h3 className="text-2xl font-bold mb-6">POV Pilot</h3>
                <div className="text-4xl font-bold mb-8 font-mono">
                  R95k<span className="text-lg text-white/30">/mo</span>
                </div>
                <ul className="space-y-4 text-sm text-white/60 mb-8 border-t border-white/10 pt-8">
                  <li className="flex items-center gap-2">
                    <SmallCheckIcon className="text-white/30" /> 1 Sandbox
                    Environment
                  </li>
                  <li className="flex items-center gap-2">
                    <SmallCheckIcon className="text-white/30" /> Up to 1M
                    executions
                  </li>
                  <li className="flex items-center gap-2">
                    <SmallCheckIcon className="text-white/30" /> 14-day log
                    retention
                  </li>
                </ul>
              </div>

              <div className="border border-cyan shadow-[0_0_30px_rgba(0,240,255,0.1)] bg-surface-light p-8 rounded-2xl relative translate-y-[-16px]">
                <div className="absolute top-0 right-8 transform translate-y-[-50%] bg-cyan text-black text-[10px] font-bold font-mono px-3 py-1 rounded-full uppercase tracking-wider">
                  Current ICP
                </div>
                <div className="font-mono text-xs text-cyan mb-4 block uppercase">
                  Core Production
                </div>
                <h3 className="text-2xl font-bold mb-6 text-white">
                  Enterprise Node
                </h3>
                <div className="text-4xl font-bold mb-8 font-mono text-white">
                  Custom
                </div>
                <ul className="space-y-4 text-sm text-white/80 mb-8 border-t border-white/10 pt-8">
                  {[
                    "Multi-tenant isolation",
                    "Unlimited execution volume",
                    "On-premise / VPC deployment",
                    "Cryptographic auditing SDK",
                  ].map((f) => (
                    <li key={f} className="flex items-center gap-2">
                      <SmallCheckIcon className="text-cyan" /> {f}
                    </li>
                  ))}
                </ul>
                <button className="w-full py-3 bg-cyan text-black font-bold rounded-lg hover:bg-white transition-colors cursor-pointer">
                  Model ROI
                </button>
              </div>

              <div className="border border-white/10 bg-surface p-8 rounded-2xl opacity-70 hover:opacity-100 transition-opacity">
                <div className="font-mono text-xs text-white/40 mb-4 block uppercase">
                  Infrastructure
                </div>
                <h3 className="text-2xl font-bold mb-6">OEM License</h3>
                <div className="text-xl font-bold mb-8 text-white/50 h-10 flex items-center">
                  Rev-Share or Flat
                </div>
                <ul className="space-y-4 text-sm text-white/60 mb-8 border-t border-white/10 pt-8">
                  {[
                    "White-labeled runtime",
                    "Embed in your iPaaS",
                    "Source code escrow",
                  ].map((f) => (
                    <li key={f} className="flex items-center gap-2">
                      <SmallCheckIcon className="text-white/30" /> {f}
                    </li>
                  ))}
                </ul>
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 10: GO TO MARKET */}
        <section id="slide-10" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="flex flex-col lg:flex-row gap-16 items-center">
              <div className="lg:w-1/3">
                <span className="font-mono text-cyan text-sm tracking-widest uppercase mb-4 block">
                  // Execution
                </span>
                <h2 className="text-4xl lg:text-5xl font-bold tracking-tight mb-6">
                  Wedge Strategy.
                </h2>
                <p className="text-white/60 text-lg">
                  Top-down direct enterprise sales targeting InfoSec, then
                  land-and-expand via platform engineering teams.
                </p>
              </div>

              <div className="lg:w-2/3 flex flex-col gap-6 relative">
                <div className="absolute left-[23px] top-4 bottom-4 w-0.5 bg-white/10" />

                {[
                  {
                    num: 1,
                    active: true,
                    title: "Phase 1: The Trust Wedge (Months 1-8)",
                    desc: "Targeting Top 5 ZAF Banks and Tier 1 Logistics. Selling to CISO/Risk Officers as an audit and compliance tool to monitor existing unmanaged webhooks.",
                  },
                  {
                    num: 2,
                    active: false,
                    title: "Phase 2: Platform Integration (Months 9-18)",
                    desc: "Once embedded, shift focus to CTO/Engineering. Guard Rail becomes the default router for all new microservice integrations, displacing legacy API gateways.",
                  },
                  {
                    num: 3,
                    active: false,
                    title: "Phase 3: The Autonomous Backbone (Year 2+)",
                    desc: "Become the foundational security layer for Enterprise AI Agents, providing the deterministic boundaries LLMs need to operate securely on production data.",
                  },
                ].map((phase) => (
                  <div
                    key={phase.num}
                    className="flex gap-8 items-start relative z-10"
                  >
                    <div
                      className={`w-12 h-12 rounded-full flex items-center justify-center font-mono font-bold shrink-0 ${phase.active ? "bg-cyan text-black border-4 border-void" : "bg-surface border-2 border-white/20 text-white"}`}
                    >
                      {phase.num}
                    </div>
                    <div className="bg-surface border border-white/5 p-6 rounded-xl w-full">
                      <h4 className="font-bold text-xl mb-2">{phase.title}</h4>
                      <p className="text-white/50 text-sm">{phase.desc}</p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* SLIDE 11: TRACTION */}
        <section
          id="slide-11"
          className="slide bg-cyan text-void selection:bg-void selection:text-cyan"
        >
          <div className="max-w-7xl mx-auto px-8 w-full">
            <div className="text-center mb-16">
              <h2 className="text-5xl font-bold tracking-tight text-void">
                Early Signal.
              </h2>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-x-8 gap-y-16">
              {[
                {
                  val: "3",
                  label: "Enterprise Pilots",
                  sub: "In FinServ & Retail",
                },
                {
                  val: "14M",
                  label: "Executions",
                  sub: "Secured in staging (last 30d)",
                },
                {
                  val: "0.8",
                  unit: "ms",
                  label: "Add. Latency",
                  sub: "P99 execution overhead",
                },
                {
                  val: "100%",
                  label: "POPIA Compliant",
                  sub: "Locally hosted instances",
                },
              ].map((stat) => (
                <div key={stat.label} className="text-center">
                  <div className="text-7xl font-bold font-mono mb-2 tracking-tighter">
                    {stat.val}
                    {stat.unit && <span className="text-3xl">{stat.unit}</span>}
                  </div>
                  <div className="font-bold uppercase tracking-wider text-sm opacity-60">
                    {stat.label}
                  </div>
                  <div className="text-xs mt-2 font-medium">{stat.sub}</div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* SLIDE 12: VISION */}
        <section id="slide-12" className="slide">
          <div className="max-w-7xl mx-auto px-8 w-full flex flex-col items-center justify-center text-center relative">
            <div className="absolute w-[150vw] left-1/2 -translate-x-1/2 top-1/2 -translate-y-1/2 font-bold text-[20vw] leading-none text-white/[0.02] whitespace-nowrap pointer-events-none select-none z-0">
              RUNTIME
            </div>

            <div className="relative z-10 max-w-4xl">
              <FingerprintIcon className="text-cyan mb-8 opacity-50 mx-auto" />
              <h2 className="text-4xl lg:text-6xl font-bold tracking-tight mb-8">
                The operating system for the{" "}
                <span className="text-cyan">autonomous enterprise.</span>
              </h2>
              <p className="text-xl text-white/50 leading-relaxed font-light">
                As software writes itself and agents talk to agents, human
                oversight will not scale. Trust cannot be assumed; it must be
                compiled, verified, and sandboxed at the moment of execution.
              </p>
            </div>
          </div>
        </section>

        {/* SLIDE 13: CLOSE / CTA */}
        <section
          id="slide-13"
          className="slide bg-surface-light border-t border-white/5"
        >
          <div className="max-w-7xl mx-auto px-8 w-full text-center">
            <div className="inline-block p-1 border border-cyan/30 rounded-lg mb-8 bg-cyan/5">
              <div className="px-6 py-2 bg-void rounded text-cyan font-mono text-sm uppercase tracking-wider shadow-inner font-bold">
                Seed Round Opening Q4
              </div>
            </div>

            <h2 className="text-6xl font-bold tracking-tight mb-12">
              Join the infrastructure.
            </h2>

            <div className="flex flex-col md:flex-row items-center justify-center gap-6 mb-24">
              <a
                href="mailto:founders@guardrail.co.za?subject=Seed%20Deck%20Inquiry"
                className="px-10 py-5 bg-white text-black font-bold rounded-lg hover:bg-cyan hover:text-black transition-colors duration-300 flex items-center gap-3 text-lg"
              >
                Contact Founders
                <svg
                  className="w-4 h-4"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M7 17L17 7M17 7H7M17 7v10"
                  />
                </svg>
              </a>
              <a
                href="#"
                className="px-10 py-5 border border-white/20 text-white font-bold rounded-lg hover:bg-white/5 transition-colors duration-300 text-lg"
              >
                Request Data Room
              </a>
            </div>

            <div className="pt-8 border-t border-white/10 flex flex-col md:flex-row justify-between items-center text-sm font-mono text-white/40">
              <div>&copy; 2024 Guard Rail Systems (Pty) Ltd.</div>
              <div className="flex gap-6 mt-4 md:mt-0">
                <a href="#" className="hover:text-cyan transition-colors">
                  Cape Town, ZAF
                </a>
                <a href="#" className="hover:text-cyan transition-colors">
                  System Status
                </a>
              </div>
            </div>
          </div>
        </section>
      </main>
    </>
  );
}

/* ── Inline SVG Icon Components ── */

function ShieldCheckIcon() {
  return (
    <svg
      className="w-12 h-12 text-cyan"
      fill="currentColor"
      viewBox="0 0 256 256"
    >
      <path d="M208,40H48A16,16,0,0,0,32,56v56c0,52.72,25.52,84.67,46.93,102.19,23.06,18.86,46,26.61,47.17,27a8,8,0,0,0,5.34.05c1.12-.38,27.85-9.49,49.36-30.07C204.12,188.4,224,157.83,224,112V56A16,16,0,0,0,208,40Zm-34.34,77.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,156.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z" />
    </svg>
  );
}

function CheckCircleIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-4 h-4 ${className}`}
      fill="currentColor"
      viewBox="0 0 256 256"
    >
      <path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z" />
    </svg>
  );
}

function ActivityIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-4 h-4 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <polyline
        points="22 12 18 12 15 21 9 3 6 12 2 12"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function WarningIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-10 h-10 ${className}`}
      fill="currentColor"
      viewBox="0 0 256 256"
    >
      <path d="M236.8,188.09,149.35,36.22h0a24.76,24.76,0,0,0-42.7,0L19.2,188.09a23.51,23.51,0,0,0,0,23.72A24.35,24.35,0,0,0,40.55,224h174.9a24.35,24.35,0,0,0,21.33-12.19A23.51,23.51,0,0,0,236.8,188.09ZM120,104a8,8,0,0,1,16,0v40a8,8,0,0,1-16,0Zm8,88a12,12,0,1,1,12-12A12,12,0,0,1,128,192Z" />
    </svg>
  );
}

function CubeIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-8 h-8 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={1.5}
        d="M21 16.5V7.8l-9-5.25L3 7.8v8.7l9 5.25 9-5.25zM3 7.8l9 5.25M12 13.05V22.5M21 7.8l-9 5.25"
      />
    </svg>
  );
}

function FileCodeIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-8 h-8 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={1.5}
        d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6zM14 2v6h6M10 12l-2 2 2 2M14 12l2 2-2 2"
      />
    </svg>
  );
}

function ReplayIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-8 h-8 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={1.5}
        d="M1 4v6h6M23 20v-6h-6M20.49 9A9 9 0 005.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 013.51 15"
      />
    </svg>
  );
}

function DatabaseIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-8 h-8 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <ellipse
        cx="12"
        cy="5"
        rx="9"
        ry="3"
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={1.5}
        d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"
      />
    </svg>
  );
}

function LightningIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-8 h-8 ${className}`}
      fill="currentColor"
      viewBox="0 0 24 24"
    >
      <path d="M13 2L3 14h9l-1 10 10-12h-9l1-10z" />
    </svg>
  );
}

function ArrowRightIcon() {
  return (
    <svg
      className="w-6 h-6"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M14 5l7 7m0 0l-7 7m7-7H3"
      />
    </svg>
  );
}

function ServerIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-8 h-8 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <rect x="2" y="2" width="20" height="8" rx="2" ry="2" strokeWidth={1.5} />
      <rect
        x="2"
        y="14"
        width="20"
        height="8"
        rx="2"
        ry="2"
        strokeWidth={1.5}
      />
      <line x1="6" y1="6" x2="6.01" y2="6" strokeWidth={2} />
      <line x1="6" y1="18" x2="6.01" y2="18" strokeWidth={2} />
    </svg>
  );
}

function CheckIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-5 h-5 ${className}`}
      fill="currentColor"
      viewBox="0 0 256 256"
    >
      <path d="M128,24A104,104,0,1,0,232,128,104.11,104.11,0,0,0,128,24Zm45.66,85.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,148.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z" />
    </svg>
  );
}

function XIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-5 h-5 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2}
        d="M6 18L18 6M6 6l12 12"
      />
    </svg>
  );
}

function SmallCheckIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-4 h-4 shrink-0 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={2.5}
        d="M5 13l4 4L19 7"
      />
    </svg>
  );
}

function FingerprintIcon({ className = "" }: { className?: string }) {
  return (
    <svg
      className={`w-16 h-16 ${className}`}
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeWidth={1.5}
        d="M12 10v4m0 4.01l.01-.011M12 2a10 10 0 0110 10c0 5.523-4.477 10-10 10S2 17.523 2 12 6.477 2 12 2zm0 4a6 6 0 016 6 6 6 0 01-6 6 6 6 0 01-6-6 6 6 0 016-6z"
      />
    </svg>
  );
}

function CompareCell({ value }: { value: boolean | string }) {
  if (value === true) return <CheckIcon className="text-green inline-block" />;
  if (value === false)
    return <XIcon className="text-crimson/50 inline-block" />;
  if (value === "partial")
    return <span className="text-yellow-500">Partial</span>;
  if (value === "offshore")
    return <span className="text-crimson/50">Offshore</span>;
  return null;
}
