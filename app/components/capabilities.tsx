"use client";

import { useInView } from "./use-in-view";
import {
  CubeIcon,
  FileCodeIcon,
  ReplayIcon,
  DatabaseIcon,
  LightningIcon,
} from "./icons";

const CAPABILITIES = [
  {
    title: "Sandboxed Runtime",
    desc: "Every external call executes in an ephemeral, isolated environment. Nothing touches your core until it\u2019s been inspected and cleared.",
    icon: CubeIcon,
    large: true,
  },
  {
    title: "Policy Engine",
    desc: "Declarative rules that inspect at the payload level. Block by parameter, pattern, or business logic. YAML-configured, version-controlled.",
    icon: FileCodeIcon,
    large: false,
  },
  {
    title: "Deterministic Replay",
    desc: "Capture full execution state. When something fails or violates policy, replay the exact environment for debugging.",
    icon: ReplayIcon,
    large: false,
  },
  {
    title: "Cryptographic Audit Trail",
    desc: "Every execution produces a tamper-proof hash. Immutable logs built for compliance audits, not just debugging.",
    icon: DatabaseIcon,
    large: false,
  },
  {
    title: "Sub-millisecond Overhead",
    desc: "Guard Rail sits in the hot path. The architecture is designed for <1ms p99 added latency.",
    icon: LightningIcon,
    large: false,
  },
];

export function Capabilities() {
  const { ref, isVisible } = useInView();

  return (
    <section id="product" className="py-32">
      <div className="max-w-7xl mx-auto px-6 lg:px-8">
        <div
          ref={ref}
          className={`fade-in-section ${isVisible ? "is-visible" : ""}`}
        >
          <h2 className="text-4xl lg:text-5xl font-bold tracking-tight mb-16">
            What Guard Rail does.
          </h2>

          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {CAPABILITIES.map((cap) => {
              const Icon = cap.icon;
              return (
                <div
                  key={cap.title}
                  className={`glass-panel p-8 rounded-xl hover:border-cyan/40 transition-all duration-300 flex flex-col justify-between ${
                    cap.large ? "md:col-span-2 lg:col-span-2" : ""
                  }`}
                >
                  <div>
                    <Icon className="text-cyan mb-5" />
                    <h3 className="text-xl font-bold mb-3">{cap.title}</h3>
                    <p className="text-sm text-white/50 leading-relaxed max-w-md">
                      {cap.desc}
                    </p>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}
