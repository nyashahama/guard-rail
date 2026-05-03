"use client";

import Link from "next/link";
import { useEffect, useRef } from "react";
import { LeadForm } from "./lead-form";

export function Pricing() {
  const ref = useRef<HTMLDivElement>(null);
  const calendarUrl = process.env.NEXT_PUBLIC_CALENDAR_URL || "/#pilot-lead-form";

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
            {"// Pilot Economics"}
          </span>
          <h2
            className="font-extrabold tracking-[-0.025em] leading-[1.0]"
            style={{ fontSize: "clamp(40px, 5vw, 72px)" }}
          >
            Pilot-first plans
            <br />
            <span className="text-white/28">for controlled rollout.</span>
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
                12-week engagement · 1 integration path
              </div>
              <hr className="border-none border-t border-white/7 mb-7" style={{ borderTop: "1px solid rgba(255,255,255,0.07)" }} />
              <ul className="flex-1 flex flex-col gap-3.5 mb-9 list-none">
                {[
                  "Single sandbox environment",
                  "Policy templates for webhook enforcement",
                  "14-day execution logs",
                ].map((f) => (
                  <li key={f} className="font-mono text-[12px] text-white/55 flex items-start gap-2.5 leading-[1.5]">
                    <span className="text-cyan/60 flex-shrink-0">/</span>
                    {f}
                  </li>
                ))}
              </ul>
              <a
                href="#pilot-lead-form"
                className="font-mono text-[11px] font-bold tracking-[0.12em] uppercase px-3.5 py-3.5 text-center no-underline block transition-all duration-300 border border-white/15 text-white/50 hover:border-white/40 hover:text-white"
              >
                Start Pilot Brief
              </a>
            </div>

            {/* Pilot Operations */}
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
                Pilot Track
              </div>
              <span className="font-mono text-[10px] tracking-[0.25em] uppercase text-cyan mb-3 block">
                Controlled Rollout
              </span>
              <div className="text-[24px] font-extrabold mb-2 tracking-[-0.01em]">Pilot Operations</div>
              <div
                className="font-mono font-bold tracking-[-0.02em] mb-1 flex items-center"
                style={{ fontSize: "28px", lineHeight: "1.3", paddingTop: "6px" }}
              >
                Custom
              </div>
              <div className="font-mono text-[11px] text-white/30 mb-8">Quarterly planning · Policy-driven flow control</div>
              <hr className="border-none border-t border-white/7 mb-7" style={{ borderTop: "1px solid rgba(255,255,255,0.07)" }} />
              <ul className="flex-1 flex flex-col gap-3.5 mb-9 list-none">
                {[
                  "Pilot check-ins and policy review",
                  "Single environment setup and handoff",
                  "Audit-ready execution metadata",
                  "Dedicated engineering support",
                ].map((f) => (
                  <li key={f} className="font-mono text-[12px] text-white/55 flex items-start gap-2.5 leading-[1.5]">
                    <span className="text-cyan/60 flex-shrink-0">/</span>
                    {f}
                  </li>
                ))}
              </ul>
              <a
                href={calendarUrl}
                className="font-mono text-[11px] font-bold tracking-[0.12em] uppercase px-3.5 py-3.5 text-center no-underline block transition-all duration-300 bg-white text-black hover:bg-cyan"
              >
                Book Pilot Call
              </a>
            </div>

            {/* OEM License */}
            <div className="obs-target bg-void px-9 py-11 flex flex-col relative opacity-80">
              <span className="font-mono text-[10px] tracking-[0.25em] uppercase text-white/30 mb-3 block">
                Infrastructure
              </span>
              <div className="text-[24px] font-extrabold mb-2 tracking-[-0.01em]">Embedded Use</div>
              <div
                className="font-mono font-bold tracking-[-0.02em] mb-1 flex items-center h-14"
                style={{ fontSize: "20px", color: "rgba(255,255,255,0.4)" }}
              >
                Pilot Program
              </div>
              <div className="font-mono text-[11px] text-white/30 mb-8">Embed where your policy enforcement is built</div>
              <hr className="border-none border-t border-white/7 mb-7" style={{ borderTop: "1px solid rgba(255,255,255,0.07)" }} />
              <ul className="flex-1 flex flex-col gap-3.5 mb-9 list-none">
                {["White-labeled runtime", "Embed in your iPaaS", "Engineering support package"].map((f) => (
                  <li key={f} className="font-mono text-[12px] text-white/55 flex items-start gap-2.5 leading-[1.5]">
                    <span className="text-cyan/60 flex-shrink-0">/</span>
                    {f}
                  </li>
                ))}
              </ul>
              <Link
                href="/docs"
                className="font-mono text-[11px] font-bold tracking-[0.12em] uppercase px-3.5 py-3.5 text-center no-underline block transition-all duration-300 border border-white/15 text-white/50 hover:border-white/40 hover:text-white"
              >
                See Integration Guide
              </Link>
            </div>
          </div>
        </div>
      </section>

      {/* ── CTA / Onboarding ── */}
      <section
        className="relative z-[1] overflow-hidden"
        style={{
          background: "var(--surface-light)",
          borderTop: "1px solid rgba(255,255,255,0.06)",
          borderBottom: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        <div
          className="absolute inset-0 pointer-events-none"
          style={{
            background: "radial-gradient(circle at 50% 100%, rgba(0,240,255,0.08) 0%, transparent 60%)",
          }}
        />
        <div className="px-20 py-[120px] max-w-[1400px] mx-auto relative">
          <div className="grid gap-14 lg:gap-10 lg:grid-cols-[1.1fr_1fr] items-start">
            <div>
              <h2
                className="font-extrabold tracking-[-0.025em] leading-[1.05] mb-4"
                style={{ fontSize: "clamp(36px, 4vw, 60px)" }}
              >
                Onboard in the right order.
              </h2>
              <p className="text-[16px] text-white/50 leading-[1.65] max-w-[520px] mt-4">
                Capture your pilot context in the form, then schedule a 30-minute sync so onboarding
                matches your environment and timeline.
              </p>
              <div className="mt-8 flex flex-col gap-3 max-w-[420px]">
                <a
                  href={calendarUrl}
                  className="font-mono text-[12px] font-bold tracking-[0.1em] uppercase text-black bg-white px-7 py-3.5 no-underline inline-flex items-center gap-2.5 transition-all duration-300 hover:bg-cyan"
                  onMouseEnter={(e) =>
                    ((e.currentTarget as HTMLElement).style.boxShadow =
                      "0 0 32px rgba(0,240,255,0.3)")
                  }
                  onMouseLeave={(e) =>
                    ((e.currentTarget as HTMLElement).style.boxShadow = "none")
                  }
                >
                  Book a Pilot Session
                  <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 17L17 7M17 7H7M17 7v10" />
                  </svg>
                </a>
                <div className="flex flex-wrap gap-3">
                <Link
                  href="/docs"
                  className="font-mono text-[11px] tracking-[0.1em] uppercase text-white/50 border border-white/15 px-6 py-3 no-underline hover:text-white hover:border-white/40 transition-all duration-200"
                >
                  Read Docs
                </Link>
                <Link
                  href="/docs/quickstart"
                  className="font-mono text-[11px] tracking-[0.1em] uppercase text-white/50 border border-white/15 px-6 py-3 no-underline hover:text-white hover:border-white/40 transition-all duration-200"
                >
                  Pilot Setup Guide
                </Link>
                <Link
                  href="/docs/webhooks-guide"
                  className="font-mono text-[11px] tracking-[0.1em] uppercase text-white/50 border border-white/15 px-6 py-3 no-underline hover:text-white hover:border-white/40 transition-all duration-200"
                >
                  Policy Integration
                </Link>
                </div>
              </div>
            </div>
            <div className="border border-white/10 px-7 py-7 bg-void">
              <LeadForm />
            </div>
          </div>
        </div>
      </section>
    </>
  );
}
