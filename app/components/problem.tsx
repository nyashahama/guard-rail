"use client";

import { useInView } from "./use-in-view";

const PAIN_POINTS = [
  {
    title: "Shadow workflows everywhere",
    desc: "Operations teams are wiring Zapier and Make into production databases. IT finds out after the incident.",
  },
  {
    title: "AI agents with root-level access",
    desc: "LLMs are calling internal tools. Nobody is validating what they\u2019re actually requesting.",
  },
  {
    title: "Compliance gaps you can\u2019t see",
    desc: "Every unaudited webhook is a POPIA violation waiting to happen. Your logs don\u2019t capture execution context.",
  },
];

export function Problem() {
  const { ref, isVisible } = useInView();

  return (
    <section className="py-32 bg-surface-light relative">
      <div className="max-w-7xl mx-auto px-6 lg:px-8">
        <div
          ref={ref}
          className={`fade-in-section ${isVisible ? "is-visible" : ""}`}
        >
          <div className="max-w-3xl mx-auto text-center mb-16">
            <h2 className="text-4xl lg:text-5xl font-bold tracking-tight mb-6">
              Your perimeter doesn&apos;t see what&apos;s running inside it.
            </h2>
            <p className="text-lg text-white/50">
              Webhooks, no-code tools, partner APIs, and AI agents are executing
              logic against your systems with no inspection, no isolation, and no
              audit trail.
            </p>
          </div>

          <div className={`grid grid-cols-1 md:grid-cols-3 gap-6 stagger-children ${isVisible ? "is-visible" : ""}`}>
            {PAIN_POINTS.map((point) => (
              <div
                key={point.title}
                className="p-8 bg-void border border-white/5 rounded-xl hover:border-cyan/40 transition-colors group"
              >
                <div className="w-1 h-8 bg-white/10 group-hover:bg-cyan rounded-full mb-6 transition-colors" />
                <h3 className="text-xl font-bold mb-3">{point.title}</h3>
                <p className="text-sm text-white/50 leading-relaxed">
                  {point.desc}
                </p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
