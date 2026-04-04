"use client";

import { useInView } from "./use-in-view";
import { CheckIcon } from "./icons";

interface Tier {
  name: string;
  label: string;
  features: string[];
  cta: string;
  ctaHref: string;
  highlighted: boolean;
}

const TIERS: Tier[] = [
  {
    name: "Pilot",
    label: "For teams evaluating Guard Rail",
    features: [
      "1 sandbox environment",
      "Up to 1M executions",
      "14-day log retention",
      "Community support",
    ],
    cta: "Request Access",
    ctaHref: "#demo",
    highlighted: false,
  },
  {
    name: "Enterprise",
    label: "For production workloads",
    features: [
      "Multi-tenant isolation",
      "Unlimited executions",
      "On-prem or VPC deployment",
      "Cryptographic audit SDK",
      "Dedicated support",
    ],
    cta: "Request Demo",
    ctaHref: "#demo",
    highlighted: true,
  },
  {
    name: "OEM",
    label: "Embed Guard Rail in your platform",
    features: [
      "White-labeled runtime",
      "Custom SLAs",
      "Source code escrow",
    ],
    cta: "Talk to Us",
    ctaHref: "#demo",
    highlighted: false,
  },
];

export function Pricing() {
  const { ref, isVisible } = useInView();

  return (
    <section id="pricing" className="py-32">
      <div className="max-w-7xl mx-auto px-6 lg:px-8">
        <div
          ref={ref}
          className={`fade-in-section ${isVisible ? "is-visible" : ""}`}
        >
          <div className="text-center mb-16">
            <h2 className="text-4xl lg:text-5xl font-bold tracking-tight mb-4">
              Simple, predictable pricing.
            </h2>
            <p className="text-white/50 text-lg">
              Enterprise-grade security that scales with your automation.
            </p>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-5xl mx-auto items-start">
            {TIERS.map((tier) => (
              <div
                key={tier.name}
                className={`rounded-2xl p-8 flex flex-col ${
                  tier.highlighted
                    ? "border border-cyan bg-surface-light shadow-[0_0_40px_rgba(0,240,255,0.08)] relative md:-translate-y-4"
                    : "border border-white/10 bg-surface"
                }`}
              >
                {tier.highlighted && (
                  <div className="absolute -top-3 left-1/2 -translate-x-1/2 bg-cyan text-black text-[10px] font-bold font-mono px-3 py-1 rounded-full uppercase tracking-wider">
                    Most Popular
                  </div>
                )}

                <div className="font-mono text-xs text-white/40 uppercase mb-2">
                  {tier.label}
                </div>
                <h3 className="text-2xl font-bold mb-8">{tier.name}</h3>

                <ul className="space-y-4 text-sm text-white/60 mb-8 flex-1">
                  {tier.features.map((f) => (
                    <li key={f} className="flex items-start gap-3">
                      <CheckIcon
                        className={
                          tier.highlighted ? "text-cyan mt-0.5" : "text-white/30 mt-0.5"
                        }
                      />
                      {f}
                    </li>
                  ))}
                </ul>

                <a
                  href={tier.ctaHref}
                  className={`w-full py-3 rounded-lg font-semibold text-sm text-center block transition-colors ${
                    tier.highlighted
                      ? "bg-cyan text-black hover:bg-white"
                      : "border border-white/20 text-white hover:bg-white/5"
                  }`}
                >
                  {tier.cta}
                </a>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
