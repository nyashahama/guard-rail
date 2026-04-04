"use client";

import { useInView } from "./use-in-view";
import { ShieldSmallIcon, GlobeIcon, LockIcon, CloudIcon } from "./icons";

const BADGES = [
  { icon: ShieldSmallIcon, label: "POPIA Compliant" },
  { icon: GlobeIcon, label: "ZA Data Residency" },
  { icon: LockIcon, label: "SOC 2" },
  { icon: CloudIcon, label: "On-Prem / VPC Deploy" },
];

const PLACEHOLDER_LOGOS = [
  "FinServ Co.",
  "National Bank",
  "Logistics Corp",
  "RetailTech",
  "InsurGroup",
];

export function Trust() {
  const { ref, isVisible } = useInView();

  return (
    <section className="py-24 border-y border-white/5">
      <div className="max-w-7xl mx-auto px-6 lg:px-8">
        <div
          ref={ref}
          className={`fade-in-section ${isVisible ? "is-visible" : ""}`}
        >
          {/* Logo bar */}
          <div className="text-center mb-16">
            <p className="text-sm font-mono text-white/30 uppercase tracking-wider mb-8">
              Built for teams at
            </p>
            <div className="flex flex-wrap items-center justify-center gap-8 lg:gap-16">
              {PLACEHOLDER_LOGOS.map((name) => (
                <span
                  key={name}
                  className="text-lg font-bold text-white/10 tracking-wider"
                >
                  {name}
                </span>
              ))}
            </div>
          </div>

          {/* Compliance badges */}
          <div className="flex flex-wrap items-center justify-center gap-6 lg:gap-10 mb-16">
            {BADGES.map((badge) => {
              const Icon = badge.icon;
              return (
                <div
                  key={badge.label}
                  className="flex items-center gap-2 text-white/40"
                >
                  <Icon className="text-white/30" />
                  <span className="text-sm font-mono">{badge.label}</span>
                </div>
              );
            })}
          </div>

          {/* Credibility line */}
          <div className="text-center">
            <p className="text-white/30 text-sm italic max-w-lg mx-auto">
              &ldquo;Built by infrastructure engineers who&apos;ve spent a decade
              securing financial systems across Southern Africa.&rdquo;
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
