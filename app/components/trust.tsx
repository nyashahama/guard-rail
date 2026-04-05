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
        <div ref={bentoRef} className="px-20 py-[120px] max-w-[1400px] mx-auto">
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
            className="mt-14"
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(3, 1fr)",
              gap: "1px",
              background: "rgba(255,255,255,0.06)",
              border: "1px solid rgba(255,255,255,0.06)",
            }}
          >
            {/* Wide cell — Payload Inspection */}
            <div
              className="obs-target bento-cell relative bg-void p-10 overflow-hidden hover:bg-surface transition-colors duration-[400ms] flex flex-row items-start justify-between gap-12 flex-wrap"
              style={{ gridColumn: "span 2" }}
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
                  policy using JSONPath field matching, and either forwards or blocks. Zero code
                  execution surface. Zero attack surface expansion.
                </p>
              </div>
              <div className="font-mono text-[11px] text-white/30 self-end whitespace-nowrap">
                env.inspect() → execution_context_id: GR-8922x
              </div>
            </div>

            {/* Declarative YAML Rules */}
            <div className="obs-target bento-cell relative bg-void p-10 overflow-hidden hover:bg-surface transition-colors duration-[400ms] flex flex-col gap-3.5">
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
            <div className="obs-target bento-cell relative bg-void p-10 overflow-hidden hover:bg-surface transition-colors duration-[400ms] flex flex-col gap-3.5">
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
            <div className="obs-target bento-cell relative bg-void p-10 overflow-hidden hover:bg-surface transition-colors duration-[400ms] flex flex-col gap-3.5">
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
              className="obs-target bento-cell relative bg-void p-10 overflow-hidden hover:bg-surface transition-colors duration-[400ms] flex flex-col gap-3.5"
              style={{ gridColumn: "span 2" }}
            >
              <div className="bento-glow" />
              <div className="grid gap-10" style={{ gridTemplateColumns: "1fr 1fr", width: "100%" }}>
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
                    hot-reload, a syntax error keeps the previous valid set active. You never
                    run unprotected.
                  </p>
                </div>
                <div>
                  <span className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan border border-cyan/25 px-2 py-[3px] self-start inline-block">
                    Compliance
                  </span>
                  <svg className="text-cyan my-3" width="32" height="32" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.5" d="M9 12l2 2 4-4M7.835 4.697a3.42 3.42 0 001.946-.806 3.42 3.42 0 014.438 0 3.42 3.42 0 001.946.806 3.42 3.42 0 013.138 3.138 3.42 3.42 0 00.806 1.946 3.42 3.42 0 010 4.438 3.42 3.42 0 00-.806 1.946 3.42 3.42 0 01-3.138 3.138 3.42 3.42 0 00-1.946.806 3.42 3.42 0 01-4.438 0 3.42 3.42 0 00-1.946-.806 3.42 3.42 0 01-3.138-3.138 3.42 3.42 0 00-.806-1.946 3.42 3.42 0 010-4.438 3.42 3.42 0 00.806-1.946 3.42 3.42 0 013.138-3.138z" />
                  </svg>
                  <div className="text-[20px] font-bold tracking-[-0.01em] mb-2">POPIA-Safe by Architecture</div>
                  <p className="font-mono text-[12.5px] text-white/45 leading-[1.7]">
                    On-premise or ZA-AWS VPC deployment. No cross-border payload transmission.
                    Data residency is a structural guarantee, not a checkbox.
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
        <div className="px-20 py-[120px] max-w-[1400px] mx-auto">
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
            className="mt-12 overflow-x-auto"
            style={{ border: "1px solid rgba(255,255,255,0.06)" }}
          >
            <table className="w-full border-collapse" style={{ minWidth: "700px" }}>
              <thead>
                <tr>
                  <th
                    className="font-mono text-[10.5px] tracking-[0.2em] uppercase text-white/40 px-7 py-5 text-left font-medium border-b border-white/6"
                    style={{ width: "38%", background: "var(--surface-light)" }}
                  >
                    Capability
                  </th>
                  <th
                    className="font-mono text-[10.5px] tracking-[0.2em] uppercase text-white/40 px-7 py-5 text-center font-medium border-b border-white/6"
                    style={{ width: "18%", background: "var(--surface-light)" }}
                  >
                    Legacy API Gateways
                  </th>
                  <th
                    className="font-mono text-[10.5px] tracking-[0.2em] uppercase text-white/40 px-7 py-5 text-center font-medium border-b border-white/6"
                    style={{ width: "18%", background: "var(--surface-light)" }}
                  >
                    In-house Middleware
                  </th>
                  <th
                    className="font-mono text-[10.5px] tracking-[0.2em] uppercase text-cyan px-7 py-5 text-center font-medium border-b border-white/6"
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
                    cap: "ZA Data Residency Guarantee",
                    legacy: "Offshore",
                    inhouse: "✓",
                    gr: "✓",
                    lStyle: "partial",
                    iStyle: "yes",
                    gStyle: "yes",
                  },
                ].map((row, i) => (
                  <tr key={i} className="group hover:[&>td]:bg-surface">
                    <td className="px-7 py-[18px] font-mono text-[12.5px] border-b border-white/4 text-white/60 transition-colors">
                      {row.cap}
                    </td>
                    <td className="px-7 py-[18px] font-mono text-[12.5px] border-b border-white/4 text-center transition-colors">
                      <span className={row.lStyle === "yes" ? "text-green block text-center" : row.lStyle === "partial" ? "text-[#FFB300] block text-center text-[11px]" : "text-white/20 block text-center"}>
                        {row.legacy}
                      </span>
                    </td>
                    <td className="px-7 py-[18px] font-mono text-[12.5px] border-b border-white/4 text-center transition-colors">
                      <span className={row.iStyle === "yes" ? "text-green block text-center" : row.iStyle === "partial" ? "text-[#FFB300] block text-center text-[11px]" : "text-white/20 block text-center"}>
                        {row.inhouse}
                      </span>
                    </td>
                    <td
                      className="px-7 py-[18px] font-mono text-[12.5px] border-b text-center transition-colors"
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
