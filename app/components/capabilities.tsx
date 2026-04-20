"use client";

import { useEffect, useRef } from "react";

export function Capabilities() {
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
    <section id="how" className="relative z-[1]">
      <div
        ref={ref}
        className="mx-auto max-w-[1400px] px-5 py-24 sm:px-8 lg:px-20 lg:py-[120px]"
      >
        <span className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan block mb-4">
          {"// The Primitive"}
        </span>
        <h2
          className="font-extrabold tracking-[-0.025em] leading-[1.0]"
          style={{ fontSize: "clamp(40px, 5vw, 72px)" }}
        >
          Zero-Trust Execution.
        </h2>
        <p className="text-[17px] text-white/55 leading-[1.65] font-light max-w-[520px] mt-4">
          Change the webhook URL. Add a YAML policy file. Guard Rail sits between
          and inspects every payload before it reaches your enterprise core.
        </p>

        {/* Flow Diagram */}
        <div className="my-14 flex flex-col items-stretch gap-6 lg:flex-row lg:items-center">
          {/* Untrusted Trigger */}
          <div className="flex flex-1 flex-col items-center gap-3.5">
            <div className="flex h-[72px] w-[72px] items-center justify-center rounded-full border border-white/15 bg-surface transition-colors duration-300 hover:border-white/40 sm:h-[88px] sm:w-[88px]">
              <svg width="32" height="32" fill="currentColor" viewBox="0 0 24 24" style={{ color: "rgba(255,255,255,0.5)" }}>
                <path d="M13 2L3 14h9l-1 10 10-12h-9l1-10z" />
              </svg>
            </div>
            <div className="text-[13px] font-semibold text-center">Untrusted Trigger</div>
            <div className="font-mono text-[10px] text-white/35 text-center leading-[1.5]">
              Webhook · Partner API<br />Zapier · AI Agent
            </div>
          </div>

          {/* Arrow → Guard Rail */}
          <div className="flex flex-shrink-0 flex-col items-center gap-2 lg:px-2">
            <div className="relative h-10 w-px bg-white/15 lg:h-px lg:w-12">
              <div className="absolute left-1/2 top-full -translate-x-1/2 border-[5px] border-transparent border-t-white/30 lg:left-auto lg:right-[-1px] lg:top-1/2 lg:-translate-y-1/2 lg:translate-x-0 lg:border-t-transparent lg:border-l-white/30" />
            </div>
            <span className="text-center font-mono text-[9px] tracking-[0.1em] text-white/30 uppercase">
              POST /v1/execute
            </span>
          </div>

          {/* Guard Rail Center */}
          <div className="flex flex-1 flex-col items-center gap-3.5">
            <div
              className="pulse-ring-circle relative flex h-[104px] w-[104px] items-center justify-center rounded-full border border-cyan/40 sm:h-[120px] sm:w-[120px]"
              style={{ background: "rgba(0,240,255,0.06)" }}
            >
              <svg width="40" height="40" fill="currentColor" viewBox="0 0 256 256" style={{ color: "var(--cyan)" }}>
                <path d="M208,40H48A16,16,0,0,0,32,56v56c0,52.72,25.52,84.67,46.93,102.19,23.06,18.86,46,26.61,47.17,27a8,8,0,0,0,5.34.05c1.12-.38,27.85-9.49,49.36-30.07C204.12,188.4,224,157.83,224,112V56A16,16,0,0,0,208,40Zm-34.34,77.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,156.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z" />
              </svg>
            </div>
            <div className="text-[13px] font-semibold text-center text-cyan">Guard Rail</div>
            <div className="font-mono text-[10px] text-white/35 text-center leading-[1.5]">Inspect · Sanitize · Log</div>
          </div>

          {/* Split arrows */}
          <div className="flex flex-shrink-0 flex-row justify-center gap-6 lg:flex-col lg:gap-2.5 lg:px-2">
            <div className="flex flex-col items-center gap-1.5">
              <div className="relative h-10 w-px bg-green lg:h-px lg:w-10">
                <div className="absolute left-1/2 top-full -translate-x-1/2 border-[5px] border-transparent border-t-green lg:left-auto lg:right-[-1px] lg:top-1/2 lg:-translate-y-1/2 lg:translate-x-0 lg:border-t-transparent lg:border-l-green" />
              </div>
              <span className="font-mono text-[9px] tracking-[0.1em] text-green uppercase">ALLOW</span>
            </div>
            <div className="flex flex-col items-center gap-1.5">
              <div className="relative h-10 w-px bg-crimson lg:h-px lg:w-10">
                <div className="absolute left-1/2 top-full -translate-x-1/2 border-[5px] border-transparent border-t-crimson lg:left-auto lg:right-[-1px] lg:top-1/2 lg:-translate-y-1/2 lg:translate-x-0 lg:border-t-transparent lg:border-l-crimson" />
              </div>
              <span className="font-mono text-[9px] tracking-[0.1em] text-crimson uppercase">BLOCK</span>
            </div>
          </div>

          {/* Outputs */}
          <div className="flex w-full flex-1 flex-col gap-2.5">
            <div className="flex items-center gap-2 border border-green/30 bg-green/4 px-4 py-3 font-mono text-[11px] font-semibold tracking-[0.08em] text-green uppercase">
              <svg width="12" height="12" fill="none" stroke="currentColor" strokeWidth={2.5} viewBox="0 0 24 24">
                <path d="M20 6L9 17l-5-5" />
              </svg>
              Forward to Enterprise Core
            </div>
            <div className="flex items-center gap-2 border border-crimson/30 bg-crimson/4 px-4 py-3 font-mono text-[11px] font-semibold tracking-[0.08em] text-crimson uppercase">
              <svg width="12" height="12" fill="none" stroke="currentColor" strokeWidth={2.5} viewBox="0 0 24 24">
                <path d="M6 18L18 6M6 6l12 12" />
              </svg>
              403 + Violation Details
            </div>
          </div>

          {/* Arrow → Enterprise Core */}
          <div className="flex flex-shrink-0 flex-col items-center gap-2 lg:px-2">
            <div className="relative h-10 w-px bg-green lg:h-px lg:w-10">
              <div className="absolute left-1/2 top-full -translate-x-1/2 border-[5px] border-transparent border-t-green lg:left-auto lg:right-[-1px] lg:top-1/2 lg:-translate-y-1/2 lg:translate-x-0 lg:border-t-transparent lg:border-l-green" />
            </div>
            <span className="text-center font-mono text-[9px] tracking-[0.1em] text-green uppercase">
              upstream response
            </span>
          </div>

          {/* Enterprise Core */}
          <div className="flex flex-1 flex-col items-center gap-3.5">
            <div
              className="flex h-[72px] w-[72px] items-center justify-center rounded-full border transition-colors duration-300 hover:border-white/40 sm:h-[88px] sm:w-[88px]"
              style={{ borderColor: "rgba(0,255,85,0.3)", background: "rgba(0,255,85,0.04)" }}
            >
              <svg width="28" height="28" fill="none" stroke="var(--green)" viewBox="0 0 24 24">
                <rect x="2" y="2" width="20" height="8" rx="2" ry="2" strokeWidth="1.5" />
                <rect x="2" y="14" width="20" height="8" rx="2" ry="2" strokeWidth="1.5" />
                <line x1="6" y1="6" x2="6.01" y2="6" strokeWidth="2" />
                <line x1="6" y1="18" x2="6.01" y2="18" strokeWidth="2" />
              </svg>
            </div>
            <div className="text-[13px] font-semibold text-center text-green">Enterprise Core</div>
            <div className="font-mono text-[10px] text-white/35 text-center leading-[1.5]">
              Internal DBs<br />Core Banking · ERP
            </div>
          </div>
        </div>

        {/* Step Cards */}
        <div
          className="mt-14 grid grid-cols-1 gap-px md:grid-cols-2 xl:grid-cols-3"
          style={{
            background: "rgba(255,255,255,0.06)",
            border: "1px solid rgba(255,255,255,0.06)",
          }}
        >
          {[
            {
              num: "01 / Intercept",
              icon: (
                <svg width="32" height="32" fill="none" stroke="var(--cyan)" viewBox="0 0 24 24" style={{ marginBottom: "16px" }}>
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
                </svg>
              ),
              title: "Your webhook, our endpoint",
              desc: (
                <>
                  Configure your automation tool to POST to{" "}
                  <code className="break-all text-cyan sm:break-normal">
                    gw.guardrail.co.za/v1/execute/&#123;route&#125;
                  </code>{" "}
                  instead of your internal system. No other code changes.
                </>
              ),
              bg: "01",
            },
            {
              num: "02 / Inspect",
              icon: (
                <svg width="32" height="32" fill="none" stroke="var(--cyan)" viewBox="0 0 24 24" style={{ marginBottom: "16px" }}>
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
                </svg>
              ),
              title: "Policy engine evaluates the payload",
              desc: "Every policy is evaluated against the request body using JSONPath field extraction. Domain checks, regex patterns, size limits, field presence — all in microseconds.",
              bg: "02",
            },
            {
              num: "03 / Verdict",
              icon: (
                <svg width="32" height="32" fill="none" stroke="var(--cyan)" viewBox="0 0 24 24" style={{ marginBottom: "16px" }}>
                  <circle cx="12" cy="12" r="10" strokeWidth="1.5" />
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M15 9l-6 6M9 9l6 6" />
                </svg>
              ),
              title: "Allow or block — with full context",
              desc: "Allowed requests forwarded with Guard Rail headers. Blocked requests return a 403 with exactly which policy matched, which field triggered it, and the value that failed.",
              bg: "03",
            },
          ].map((step, index) => (
            <div
              key={step.num}
              className={`obs-target relative overflow-hidden bg-void px-6 py-8 transition-colors duration-300 hover:bg-surface sm:px-8 sm:py-10 ${
                index === 2 ? "md:col-span-2 xl:col-span-1" : ""
              }`}
            >
              <span className="font-mono text-[11px] tracking-[0.2em] text-cyan mb-5 block">{step.num}</span>
              {step.icon}
              <div className="text-[20px] font-bold mb-3.5">{step.title}</div>
              <div className="font-mono text-[12.5px] text-white/45 leading-[1.7]">
                {step.desc}
              </div>
              <div
                className="absolute bottom-[-20px] right-4 font-mono font-extrabold leading-[1] select-none pointer-events-none"
                style={{ fontSize: "100px", color: "rgba(0,240,255,0.03)" }}
              >
                {step.bg}
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
