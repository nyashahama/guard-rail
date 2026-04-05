"use client";

import { useEffect, useRef } from "react";

export function Pricing() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((e) => {
          if (e.isIntersecting) {
            e.target.classList.add("visible");
            observer.unobserve(e.target);
          }
        });
      },
      { threshold: 0.08 }
    );
    ref.current?.querySelectorAll(".obs-target").forEach((el) => observer.observe(el));
    return () => observer.disconnect();
  }, []);

  return (
    <>
      {/* ── PRICING ── */}
      <section id="pricing" className="relative z-[1]">
        <div ref={ref} className="px-20 py-[120px] max-w-[1400px] mx-auto">
          <span className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan block mb-4">
            // Value Extraction
          </span>
          <h2
            className="font-extrabold tracking-[-0.025em] leading-[1.0]"
            style={{ fontSize: "clamp(40px, 5vw, 72px)" }}
          >
            Predictable compute pricing
            <br />
            <span className="text-white/28">aligned to enterprise scale.</span>
          </h2>

          {/* Pricing Grid */}
          <div
            className="mt-14"
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(3, 1fr)",
              gap: "1px",
              background: "rgba(255,255,255,0.06)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          >
            {/* POV Pilot */}
            <div className="obs-target bg-void px-9 py-11 flex flex-col relative opacity-80">
              <span className="font-mono text-[10px] tracking-[0.25em] uppercase text-white/30 mb-3 block">
                Proof of Value
              </span>
              <div className="text-[24px] font-extrabold mb-2 tracking-[-0.01em]">POV Pilot</div>
              <div className="font-mono text-[38px] font-bold tracking-[-0.02em] mb-1">
                R95k<span className="text-[16px] text-white/35 font-normal">/mo</span>
              </div>
              <div className="font-mono text-[11px] text-white/30 mb-8">
                12-week engagement · 1 environment
              </div>
              <hr className="border-none border-t border-white/7 mb-7" style={{ borderTop: "1px solid rgba(255,255,255,0.07)" }} />
              <ul className="flex-1 flex flex-col gap-3.5 mb-9 list-none">
                {["1 Sandbox Environment", "Up to 1M executions", "14-day log retention"].map((f) => (
                  <li key={f} className="font-mono text-[12px] text-white/55 flex items-start gap-2.5 leading-[1.5]">
                    <span className="text-cyan/60 flex-shrink-0">//</span>
                    {f}
                  </li>
                ))}
              </ul>
              <a
                href="mailto:founders@guardrail.co.za?subject=POV Pilot"
                className="font-mono text-[11px] font-bold tracking-[0.12em] uppercase px-3.5 py-3.5 text-center no-underline block transition-all duration-300 border border-white/15 text-white/50 hover:border-white/40 hover:text-white"
              >
                Start Pilot
              </a>
            </div>

            {/* Enterprise Node — Featured */}
            <div
              className="obs-target px-9 py-11 flex flex-col relative"
              style={{
                background: "var(--surface)",
                borderTop: "2px solid var(--cyan)",
              }}
            >
              {/* Badge */}
              <div
                className="absolute top-0 right-8 font-mono text-[10px] font-bold tracking-[0.15em] uppercase text-black px-3 py-1 rounded-full"
                style={{
                  transform: "translateY(-50%)",
                  background: "var(--cyan)",
                }}
              >
                Current ICP
              </div>
              <span className="font-mono text-[10px] tracking-[0.25em] uppercase text-cyan mb-3 block">
                Core Production
              </span>
              <div className="text-[24px] font-extrabold mb-2 tracking-[-0.01em]">Enterprise Node</div>
              <div
                className="font-mono font-bold tracking-[-0.02em] mb-1 flex items-center"
                style={{ fontSize: "28px", lineHeight: "1.3", paddingTop: "6px" }}
              >
                Custom
              </div>
              <div className="font-mono text-[11px] text-white/30 mb-8">
                Annual contract · multi-environment
              </div>
              <hr className="border-none border-t border-white/7 mb-7" style={{ borderTop: "1px solid rgba(255,255,255,0.07)" }} />
              <ul className="flex-1 flex flex-col gap-3.5 mb-9 list-none">
                {[
                  "Multi-tenant isolation",
                  "Unlimited execution volume",
                  "On-premise / VPC deployment",
                  "Cryptographic auditing SDK",
                ].map((f) => (
                  <li key={f} className="font-mono text-[12px] text-white/55 flex items-start gap-2.5 leading-[1.5]">
                    <span className="text-cyan/60 flex-shrink-0">//</span>
                    {f}
                  </li>
                ))}
              </ul>
              <a
                href="mailto:founders@guardrail.co.za?subject=Enterprise"
                className="font-mono text-[11px] font-bold tracking-[0.12em] uppercase px-3.5 py-3.5 text-center no-underline block transition-all duration-300 bg-white text-black hover:bg-cyan"
                onMouseEnter={(e) =>
                  ((e.currentTarget as HTMLElement).style.boxShadow =
                    "0 0 24px rgba(0,240,255,0.3)")
                }
                onMouseLeave={(e) =>
                  ((e.currentTarget as HTMLElement).style.boxShadow = "none")
                }
              >
                Model ROI
              </a>
            </div>

            {/* OEM License */}
            <div className="obs-target bg-void px-9 py-11 flex flex-col relative opacity-80">
              <span className="font-mono text-[10px] tracking-[0.25em] uppercase text-white/30 mb-3 block">
                Infrastructure
              </span>
              <div className="text-[24px] font-extrabold mb-2 tracking-[-0.01em]">OEM License</div>
              <div
                className="font-mono font-bold tracking-[-0.02em] mb-1 flex items-center h-14"
                style={{ fontSize: "20px", color: "rgba(255,255,255,0.4)" }}
              >
                Rev-Share or Flat
              </div>
              <div className="font-mono text-[11px] text-white/30 mb-8">Embed in your iPaaS</div>
              <hr className="border-none border-t border-white/7 mb-7" style={{ borderTop: "1px solid rgba(255,255,255,0.07)" }} />
              <ul className="flex-1 flex flex-col gap-3.5 mb-9 list-none">
                {["White-labeled runtime", "Embed in your iPaaS", "Source code escrow"].map((f) => (
                  <li key={f} className="font-mono text-[12px] text-white/55 flex items-start gap-2.5 leading-[1.5]">
                    <span className="text-cyan/60 flex-shrink-0">//</span>
                    {f}
                  </li>
                ))}
              </ul>
              <a
                href="mailto:founders@guardrail.co.za?subject=OEM"
                className="font-mono text-[11px] font-bold tracking-[0.12em] uppercase px-3.5 py-3.5 text-center no-underline block transition-all duration-300 border border-white/15 text-white/50 hover:border-white/40 hover:text-white"
              >
                Talk to us
              </a>
            </div>
          </div>
        </div>
      </section>

      {/* ── CTA ── */}
      <section
        className="relative z-[1] overflow-hidden"
        style={{
          background: "var(--surface-light)",
          borderTop: "1px solid rgba(255,255,255,0.06)",
          borderBottom: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        {/* CTA glow */}
        <div
          className="absolute inset-0 pointer-events-none"
          style={{
            background: "radial-gradient(circle at 50% 100%, rgba(0,240,255,0.08) 0%, transparent 60%)",
          }}
        />
        <div className="px-20 py-[120px] max-w-[1400px] mx-auto relative">
          <div className="grid gap-20 items-center" style={{ gridTemplateColumns: "1fr auto" }}>
            <div>
              <h2
                className="font-extrabold tracking-[-0.025em] leading-[1.05] mb-4"
                style={{ fontSize: "clamp(36px, 4vw, 60px)" }}
              >
                Join the infrastructure.
              </h2>
              <p className="text-[16px] text-white/50 leading-[1.65] max-w-[500px] mt-4">
                Book a 30-minute call and we&apos;ll configure your first policy live. No SDK.
                No agents. No infrastructure overhaul.
              </p>
            </div>
            <div className="flex flex-col gap-3 items-start flex-shrink-0">
              <a
                href="mailto:founders@guardrail.co.za?subject=Pilot Request"
                className="font-mono text-[12px] font-bold tracking-[0.1em] uppercase text-black bg-white px-7 py-3.5 no-underline inline-flex items-center gap-2.5 transition-all duration-300 hover:bg-cyan"
                onMouseEnter={(e) =>
                  ((e.currentTarget as HTMLElement).style.boxShadow =
                    "0 0 32px rgba(0,240,255,0.3)")
                }
                onMouseLeave={(e) =>
                  ((e.currentTarget as HTMLElement).style.boxShadow = "none")
                }
              >
                Contact Founders
                <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 17L17 7M17 7H7M17 7v10" />
                </svg>
              </a>
              <a
                href="#"
                className="font-mono text-[12px] tracking-[0.1em] uppercase text-white/50 border border-white/15 px-7 py-3.5 no-underline hover:text-white hover:border-white/40 transition-all duration-200"
              >
                Request Data Room
              </a>
              <span className="font-mono text-[10px] tracking-[0.1em] text-white/25 mt-1">
                founders@guardrail.co.za
              </span>
            </div>
          </div>
        </div>
      </section>
    </>
  );
}
