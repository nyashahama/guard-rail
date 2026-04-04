"use client";

import { useState } from "react";
import { useInView } from "./use-in-view";
import { LightningIcon, ShieldSmallIcon, ServerIcon } from "./icons";

const TABS = [
  {
    label: "Policy (YAML)",
    content: `# guardrail-policy.yaml
rules:
  - name: block-external-callbacks
    trigger: webhook.inbound
    match:
      field: callback
      condition: domain_not_in
      values:
        - "*.guardrail.co.za"
        - "*.internal.bank.za"
    action: block
    severity: critical
    log: true`,
  },
  {
    label: "Execution Log (JSON)",
    content: `{
  "event_id": "GR-EXE-449281",
  "timestamp": "2026-04-04T14:32:01.004Z",
  "trigger": "webhook.inbound",
  "source_ip": "41.13.22.91",
  "policy": "block-external-callbacks",
  "verdict": "BLOCKED",
  "violation_field": "callback",
  "violation_value": "https://evil.sh/exfil",
  "hash": "9f3a8b...c7e1",
  "latency_ms": 0.42
}`,
  },
  {
    label: "Integration (cURL)",
    content: `# Route any webhook through Guard Rail
curl -X POST https://gw.guardrail.co.za/v1/execute \\
  -H "Authorization: Bearer gr_live_sk_..." \\
  -H "Content-Type: application/json" \\
  -d @payload.json`,
  },
];

export function Architecture() {
  const { ref, isVisible } = useInView(0.1);
  const [activeTab, setActiveTab] = useState(0);

  return (
    <section id="architecture" className="py-32 bg-surface-light">
      <div className="max-w-7xl mx-auto px-6 lg:px-8">
        <div
          ref={ref}
          className={`fade-in-section ${isVisible ? "is-visible" : ""}`}
        >
          <h2 className="text-4xl lg:text-5xl font-bold tracking-tight mb-20 text-center">
            How it fits into your stack.
          </h2>

          {/* Flow Diagram */}
          <div className="flex flex-col md:flex-row items-center justify-center gap-6 lg:gap-12 mb-24">
            {/* Untrusted Trigger */}
            <div
              className={`flex flex-col items-center transition-all duration-700 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"
              }`}
            >
              <div className="w-20 h-20 rounded-full border border-white/20 bg-surface flex items-center justify-center mb-3">
                <LightningIcon className="text-white/60 w-6 h-6" />
              </div>
              <h4 className="font-bold text-sm mb-1">Untrusted Trigger</h4>
              <span className="font-mono text-[11px] text-white/40 text-center">
                Webhook, Partner API,
                <br />
                AI Agent
              </span>
            </div>

            {/* Arrow 1 */}
            <svg
              className={`hidden md:block w-24 h-4 transition-all duration-700 delay-300 ${
                isVisible ? "opacity-100" : "opacity-0"
              }`}
              viewBox="0 0 96 16"
            >
              <line
                x1="0" y1="8" x2="80" y2="8"
                stroke="rgba(255,255,255,0.2)"
                strokeWidth="1.5"
                className={`flow-line ${isVisible ? "is-visible" : ""}`}
                style={{ animationDelay: "0.4s" }}
              />
              <polygon points="80,3 92,8 80,13" fill="#00F0FF" className={`transition-opacity duration-300 ${isVisible ? "opacity-100" : "opacity-0"}`} style={{ transitionDelay: "1s" }} />
            </svg>

            {/* Guard Rail (center) */}
            <div
              className={`flex flex-col items-center transition-all duration-700 delay-500 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"
              }`}
            >
              <div className={`w-28 h-28 rounded-2xl glass-panel border-cyan/50 flex flex-col items-center justify-center mb-3 ${isVisible ? "node-active" : ""}`}>
                <ShieldSmallIcon className="text-cyan w-8 h-8" />
                <span className="font-mono text-[9px] font-bold tracking-widest uppercase mt-1.5">
                  Guard Rail
                </span>
              </div>
              <div className="flex gap-2 font-mono text-[10px]">
                <span className="px-2 py-1 bg-surface border border-white/10 rounded">
                  Inspect
                </span>
                <span className="px-2 py-1 bg-surface border border-white/10 rounded">
                  Enforce
                </span>
                <span className="px-2 py-1 bg-surface border border-white/10 rounded">
                  Log
                </span>
              </div>
            </div>

            {/* Arrow 2 */}
            <svg
              className={`hidden md:block w-24 h-4 transition-all duration-700 delay-700 ${
                isVisible ? "opacity-100" : "opacity-0"
              }`}
              viewBox="0 0 96 16"
            >
              <line
                x1="0" y1="8" x2="80" y2="8"
                stroke="rgba(255,255,255,0.2)"
                strokeWidth="1.5"
                className={`flow-line ${isVisible ? "is-visible" : ""}`}
                style={{ animationDelay: "0.8s" }}
              />
              <polygon points="80,3 92,8 80,13" fill="#00FF55" className={`transition-opacity duration-300 ${isVisible ? "opacity-100" : "opacity-0"}`} style={{ transitionDelay: "1.4s" }} />
            </svg>

            {/* Enterprise Core */}
            <div
              className={`flex flex-col items-center transition-all duration-700 delay-1000 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"
              }`}
            >
              <div className="w-20 h-20 rounded-lg border border-green/30 bg-green/5 flex items-center justify-center mb-3">
                <ServerIcon className="text-green w-6 h-6" />
              </div>
              <h4 className="font-bold text-sm mb-1">Enterprise Core</h4>
              <span className="font-mono text-[11px] text-white/40 text-center">
                Internal DBs, Core Systems,
                <br />
                ERPs
              </span>
            </div>
          </div>

          {/* Tabbed Code Block */}
          <div className="max-w-3xl mx-auto">
            <div className="bg-void rounded-xl border border-white/10 shadow-2xl overflow-hidden">
              {/* Tab bar */}
              <div className="flex border-b border-white/10">
                {TABS.map((tab, i) => (
                  <button
                    key={tab.label}
                    onClick={() => setActiveTab(i)}
                    className={`px-5 py-3 font-mono text-xs transition-colors cursor-pointer ${
                      activeTab === i
                        ? "text-cyan border-b-2 border-cyan bg-white/5"
                        : "text-white/40 hover:text-white/60"
                    }`}
                  >
                    {tab.label}
                  </button>
                ))}
              </div>

              {/* Code content */}
              <pre className="p-6 font-mono text-[13px] leading-relaxed text-white/70 overflow-x-auto">
                {TABS[activeTab].content}
              </pre>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
