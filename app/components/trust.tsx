"use client";

import { useEffect, useRef } from "react";

export function Trust() {
  const bentoRef = useRef<HTMLDivElement>(null);

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
    bentoRef.current?.querySelectorAll(".obs-target").forEach((el) => observer.observe(el));
    return () => observer.disconnect();
  }, []);

  return (
    <>
      {/* ── FEATURES / BENTO ── */}
      <section id="features" className="relative z-[1]">
        <div
          ref={bentoRef}
          className="mx-auto max-w-[1400px] px-5 py-24 sm:px-8 lg:px-20 lg:py-[120px]"
        >
          <span className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan block mb-4">
            {"// Capabilities"}
          </span>
          <h2
            className="font-extrabold tracking-[-0.025em] leading-[1.0]"
            style={{ fontSize: "clamp(40px, 5vw, 72px)" }}
          >
            Designed for
            <br />
            <span className="text-white/28">Resilience.</span>
          </h2>

          {/* Bento Grid */}
          <div
            className="mt-14 grid grid-cols-1 gap-px md:grid-cols-2 xl:grid-cols-3"
            style={{
              background: "rgba(255,255,255,0.06)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          >
            {/* Wide cell — Payload Inspection */}
            <div
              className="obs-target bento-cell relative flex flex-col items-start justify-between gap-8 overflow-hidden bg-void p-6 transition-colors duration-[400ms] hover:bg-surface md:col-span-2 sm:p-8 lg:flex-row lg:gap-12 lg:p-10 xl:col-span-2"
            >
              <div className="bento-glow" />
              <div>
                <span
                  className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan border border-cyan/25 px-2 py-[3px] self-start inline-block"
                >
                  Sandboxed Runtime
                </span>
                <svg
                  className="text-cyan my-3"
                  width="32" height="32" fill="none" stroke="currentColor" viewBox="0 0 24 24"
                >
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M21 16.5V7.8l-9-5.25L3 7.8v8.7l9 5.25 9-5.25zM3 7.8l9 5.25M12 13.05V22.5M21 7.8l-9 5.25" />
                </svg>
                <div className="text-[20px] font-bold tracking-[-0.01em] mb-2">Payload Inspection</div>
                <p className="font-mono text-[12.5px] text-white/45 leading-[1.7] max-w-[380px]">
                  Guard Rail receives your webhook, inspects the payload against every configured
                  policy using JSONPath field matching, and either forwards or blocks the request.
                  No custom application code runs inside the policy path.
                </p>
              </div>
              <div className="self-start font-mono text-[11px] text-white/30 break-all lg:self-end lg:text-right lg:break-normal">
                env.inspect() → execution_context_id: GR-8922x
              </div>
            </div>

            {/* Declarative YAML Rules */}
            <div className="obs-target bento-cell relative flex flex-col gap-3.5 overflow-hidden bg-void p-6 transition-colors duration-[400ms] hover:bg-surface sm:p-8 lg:p-10">
              <div className="bento-glow" />
              <span className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan border border-cyan/25 px-2 py-[3px] self-start">
                Policy Engine
              </span>
              <svg className="text-cyan" width="32" height="32" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6zM14 2v6h6M10 12l-2 2 2 2M14 12l2 2-2 2" />
              </svg>
              <div className="text-[20px] font-bold tracking-[-0.01em]">Declarative YAML Rules</div>
              <p className="font-mono text-[12.5px] text-white/45 leading-[1.7] max-w-[380px]">
                Block malicious payloads at the field level. 11 condition types, JSONPath
                targeting, hot-reload on file change.
              </p>
              <div className="w-full h-[3px] rounded-[2px] overflow-hidden mt-auto" style={{ background: "rgba(255,255,255,0.06)" }}>
                <div className="w-3/4 h-full bg-cyan rounded-[2px]" />
              </div>
            </div>

            {/* Cryptographic Audit Logs */}
            <div className="obs-target bento-cell relative flex flex-col gap-3.5 overflow-hidden bg-void p-6 transition-colors duration-[400ms] hover:bg-surface sm:p-8 lg:p-10">
              <div className="bento-glow" />
              <span className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan border border-cyan/25 px-2 py-[3px] self-start">
                Audit Trail
              </span>
              <svg className="text-cyan" width="32" height="32" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <ellipse cx="12" cy="5" rx="9" ry="3" strokeWidth="1.5" />
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
              </svg>
              <div className="text-[20px] font-bold tracking-[-0.01em]">Cryptographic Audit Logs</div>
              <div
                className="bg-void border border-white/5 p-4 font-mono text-[10.5px] leading-[1.9] mt-2 flex-1"
              >
                <div className="text-white/25">[INFO] Checksum verified</div>
                <div className="text-[#FFB300]">[WARN] PII detected, masking...</div>
                <div className="text-green">[PASS] Block #9924 committed</div>
                <div className="text-crimson">[BLOCK] domain_not_in triggered</div>
                <div className="text-white/25">[INFO] upstream → 200 OK</div>
              </div>
            </div>

            {/* Deterministic Replay */}
            <div className="obs-target bento-cell relative flex flex-col gap-3.5 overflow-hidden bg-void p-6 transition-colors duration-[400ms] hover:bg-surface sm:p-8 lg:p-10">
              <div className="bento-glow" />
              <span className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan border border-cyan/25 px-2 py-[3px] self-start">
                Replay Engine
              </span>
              <svg className="text-cyan" width="32" height="32" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M1 4v6h6M23 20v-6h-6M20.49 9A9 9 0 005.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 013.51 15" />
              </svg>
              <div className="text-[20px] font-bold tracking-[-0.01em]">Deterministic Replay</div>
              <p className="font-mono text-[12.5px] text-white/45 leading-[1.7] max-w-[380px]">
                Capture full request state. Replay exact execution for debugging — against
                current or modified policies.
              </p>
            </div>

            {/* Wide cell — Safety + Compliance */}
            <div
              className="obs-target bento-cell relative flex flex-col gap-3.5 overflow-hidden bg-void p-6 transition-colors duration-[400ms] hover:bg-surface md:col-span-2 sm:p-8 lg:p-10 xl:col-span-2"
            >
              <div className="bento-glow" />
              <div className="grid w-full grid-cols-1 gap-8 lg:grid-cols-2 lg:gap-10">
                <div>
                  <span className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan border border-cyan/25 px-2 py-[3px] self-start inline-block">
                    Safety
                  </span>
                  <svg className="text-cyan my-3" width="32" height="32" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
                  </svg>
                  <div className="text-[20px] font-bold tracking-[-0.01em] mb-2">Fail-Closed by Design</div>
                  <p className="font-mono text-[12.5px] text-white/45 leading-[1.7]">
                    Guard Rail refuses to start if policy files reference a missing name. On
                    hot-reload, a syntax error keeps the last valid set active so an invalid
                    update does not silently disable inspection.
                  </p>
                </div>
                <div>
                  <span className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan border border-cyan/25 px-2 py-[3px] self-start inline-block">
                    Compliance
                  </span>
                  <svg className="text-cyan my-3" width="32" height="32" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z" />
                  </svg>
                  <div className="text-[20px] font-bold tracking-[-0.01em] mb-2">
                    Deployment Boundaries That Support POPIA
                  </div>
                  <p className="font-mono text-[12.5px] text-white/45 leading-[1.7]">
                    On-premise or single-region ZA AWS VPC deployment gives teams a path to keep
                    payload handling within their chosen boundary. Final compliance still depends
                    on deployment, policy, and operational controls.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <hr className="border-none border-t border-white/6 relative z-[1]" style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }} />

      {/* ── COMPARISON ── */}
      <section
        className="relative z-[1]"
        style={{
          background: "var(--surface-light)",
          borderTop: "1px solid rgba(255,255,255,0.04)",
          borderBottom: "1px solid rgba(255,255,255,0.04)",
        }}
      >
        <div className="mx-auto max-w-[1400px] px-5 py-24 sm:px-8 lg:px-20 lg:py-[120px]">
          <span className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan block mb-4">
            {"// The Shift in Paradigm"}
          </span>
          <h2
            className="font-extrabold tracking-[-0.025em] leading-[1.0]"
            style={{ fontSize: "clamp(40px, 5vw, 72px)" }}
          >
            What standard
            <br />
            <span className="text-white/28">gateways miss.</span>
          </h2>

          {/* Comparison Table */}
          <div
            className="trust-scroll-area mt-12 -mx-5 overflow-x-auto px-5 sm:mx-0 sm:px-0"
            style={{ border: "1px solid rgba(255,255,255,0.06)" }}
          >
            <table className="w-full min-w-[640px] border-collapse sm:min-w-[700px]">
              <thead>
                <tr>
                  <th
                    className="border-b border-white/6 px-4 py-4 text-left font-mono text-[10.5px] font-medium tracking-[0.2em] text-white/40 uppercase sm:px-7 sm:py-5"
                    style={{ width: "38%", background: "var(--surface-light)" }}
                  >
                    Capability
                  </th>
                  <th
                    className="border-b border-white/6 px-4 py-4 text-center font-mono text-[10.5px] font-medium tracking-[0.2em] text-white/40 uppercase sm:px-7 sm:py-5"
                    style={{ width: "18%", background: "var(--surface-light)" }}
                  >
                    Legacy API Gateways
                  </th>
                  <th
                    className="border-b border-white/6 px-4 py-4 text-center font-mono text-[10.5px] font-medium tracking-[0.2em] text-white/40 uppercase sm:px-7 sm:py-5"
                    style={{ width: "18%", background: "var(--surface-light)" }}
                  >
                    In-house Middleware
                  </th>
                  <th
                    className="border-b border-white/6 px-4 py-4 text-center font-mono text-[10.5px] font-medium tracking-[0.2em] text-cyan uppercase sm:px-7 sm:py-5"
                    style={{
                      width: "26%",
                      background: "rgba(0,240,255,0.05)",
                      borderTop: "2px solid rgba(0,240,255,0.5)",
                    }}
                  >
                    Guard Rail
                  </th>
                </tr>
              </thead>
              <tbody>
                {[
                  {
                    cap: "Header & Token Auth",
                    legacy: "✓",
                    inhouse: "✓",
                    gr: "✓",
                    lStyle: "yes",
                    iStyle: "yes",
                    gStyle: "yes",
                  },
                  {
                    cap: "Deep Payload Logic Inspection",
                    legacy: "—",
                    inhouse: "Partial",
                    gr: "✓",
                    lStyle: "no",
                    iStyle: "partial",
                    gStyle: "yes",
                  },
                  {
                    cap: "Sandboxed Execution Environment",
                    legacy: "—",
                    inhouse: "—",
                    gr: "✓",
                    lStyle: "no",
                    iStyle: "no",
                    gStyle: "yes",
                  },
                  {
                    cap: "Deterministic Error Replay",
                    legacy: "—",
                    inhouse: "—",
                    gr: "✓",
                    lStyle: "no",
                    iStyle: "no",
                    gStyle: "yes",
                  },
                  {
                    cap: "ZA Residency Controls",
                    legacy: "Vendor-specific",
                    inhouse: "✓",
                    gr: "✓",
                    lStyle: "partial",
                    iStyle: "yes",
                    gStyle: "yes",
                  },
                ].map((row, i) => (
                  <tr key={i} className="group hover:[&>td]:bg-surface">
                    <td className="border-b border-white/4 px-4 py-4 font-mono text-[12px] text-white/60 transition-colors sm:px-7 sm:py-[18px] sm:text-[12.5px]">
                      {row.cap}
                    </td>
                    <td className="border-b border-white/4 px-4 py-4 text-center font-mono text-[12px] transition-colors sm:px-7 sm:py-[18px] sm:text-[12.5px]">
                      <span className={row.lStyle === "yes" ? "text-green block text-center" : row.lStyle === "partial" ? "text-[#FFB300] block text-center text-[11px]" : "text-white/20 block text-center"}>
                        {row.legacy}
                      </span>
                    </td>
                    <td className="border-b border-white/4 px-4 py-4 text-center font-mono text-[12px] transition-colors sm:px-7 sm:py-[18px] sm:text-[12.5px]">
                      <span className={row.iStyle === "yes" ? "text-green block text-center" : row.iStyle === "partial" ? "text-[#FFB300] block text-center text-[11px]" : "text-white/20 block text-center"}>
                        {row.inhouse}
                      </span>
                    </td>
                    <td
                      className="border-b px-4 py-4 text-center font-mono text-[12px] transition-colors sm:px-7 sm:py-[18px] sm:text-[12.5px]"
                      style={{
                        background: "rgba(0,240,255,0.03)",
                        borderLeft: "1px solid rgba(0,240,255,0.1)",
                        borderRight: "1px solid rgba(0,240,255,0.1)",
                        borderBottom: i === 4 ? "1px solid rgba(0,240,255,0.1)" : "1px solid rgba(255,255,255,0.04)",
                      }}
                    >
                      <span className="text-green block text-center">{row.gr}</span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>
    </>
  );
}
