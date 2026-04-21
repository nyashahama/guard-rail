"use client";

import { useEffect, useRef } from "react";

export function Problem() {
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
    <section className="relative z-[1]">
      <div ref={ref} className="px-6 sm:px-8 lg:px-20 py-20 sm:py-[120px] max-w-[1400px] mx-auto">
        <div className="grid gap-10 lg:gap-16 items-start lg:grid-cols-2">
          {/* Left — Copy */}
          <div>
            <span className="font-mono text-[11px] tracking-[0.25em] uppercase text-crimson block mb-4">
              {"// Risk Vector"}
            </span>
            <h2
              className="font-extrabold tracking-[-0.025em] leading-[1.0] mb-6"
              style={{ fontSize: "clamp(36px, 5vw, 72px)" }}
            >
              Automation is scaling.
              <br />
              <span className="text-white/28">Control is not.</span>
            </h2>
            <p className="text-[17px] text-white/55 leading-[1.65] font-light max-w-[460px] mt-6">
              Operations now move through Zapier, Make, custom scripts, and AI
              agents. Those flows still need request-level inspection because
              perimeter controls do not understand what happens inside the
              workflow.
            </p>
            <p className="text-[17px] text-white/55 leading-[1.65] font-light max-w-[460px] mt-4">
              When an unverified third-party app executes a workflow across your
              internal API, you don&apos;t have a firewall problem.{" "}
              <strong className="text-white font-semibold">
                You have a runtime problem.
              </strong>
            </p>
          </div>

          {/* Right — Stat Cards */}
          <div className="grid gap-4">
            {/* Fast-moving sprawl */}
            <div className="obs-target relative bg-surface border border-white/5 p-6 sm:p-10 overflow-hidden transition-colors duration-300 hover:border-crimson/40">
              <div
                className="absolute top-0 right-0 w-[60px] h-[60px] bg-crimson/8"
                style={{ borderRadius: "0 0 0 100%", filter: "blur(20px)" }}
              />
              <div className="font-mono text-[15px] tracking-[0.18em] uppercase text-crimson mb-3">
                Fast-moving sprawl
              </div>
              <div className="text-[18px] font-semibold mb-2">More routes, more blind spots</div>
              <div className="font-mono text-[12px] text-white/40 leading-[1.6] max-w-[34ch]">
                New integrations and workflow tools arrive faster than teams can
                review them, which makes audit trails harder to trust.
              </div>
              <div className="absolute bottom-5 right-5 text-crimson/40">
                <svg width="40" height="40" fill="currentColor" viewBox="0 0 256 256">
                  <path d="M236.8,188.09,149.35,36.22h0a24.76,24.76,0,0,0-42.7,0L19.2,188.09a23.51,23.51,0,0,0,0,23.72A24.35,24.35,0,0,0,40.55,224h174.9a24.35,24.35,0,0,0,21.33-12.19A23.51,23.51,0,0,0,236.8,188.09ZM120,104a8,8,0,0,1,16,0v40a8,8,0,0,1-16,0Zm8,88a12,12,0,1,1,12-12A12,12,0,0,1,128,192Z" />
                </svg>
              </div>
            </div>

            {/* Unverified third-party logic */}
            <div className="obs-target relative bg-surface border border-white/5 p-6 sm:p-10 overflow-hidden transition-colors duration-300 hover:border-crimson/40">
              <div
                className="absolute top-0 right-0 w-[60px] h-[60px] bg-crimson/8"
                style={{ borderRadius: "0 0 0 100%", filter: "blur(20px)" }}
              />
              <div className="font-mono text-[15px] tracking-[0.18em] uppercase text-white/70 mb-3">
                Unverified third-party logic
              </div>
              <div className="text-[18px] font-semibold mb-2">External steps still need inspection</div>
              <div className="font-mono text-[12px] text-white/40 leading-[1.6] max-w-[34ch]">
                External webhooks and partner automations can move critical data
                without any payload-level check unless the runtime is in line.
              </div>
            </div>

            {/* Shadow Workflows */}
            <div className="obs-target relative bg-surface border border-white/5 overflow-hidden transition-colors duration-300 hover:border-crimson/40 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 px-6 sm:px-9 py-6">
              <div
                className="absolute top-0 right-0 w-[60px] h-[60px] bg-crimson/8"
                style={{ borderRadius: "0 0 0 100%", filter: "blur(20px)" }}
              />
              <div>
                <div className="text-[18px] font-semibold text-crimson">Shadow Workflows</div>
                <div className="font-mono text-[12px] text-white/40 mt-1">
                  Operations bypassing central review and control.
                </div>
              </div>
              <div className="flex-shrink-0 text-crimson/40">
                <svg width="40" height="40" fill="currentColor" viewBox="0 0 256 256">
                  <path d="M236.8,188.09,149.35,36.22h0a24.76,24.76,0,0,0-42.7,0L19.2,188.09a23.51,23.51,0,0,0,0,23.72A24.35,24.35,0,0,0,40.55,224h174.9a24.35,24.35,0,0,0,21.33-12.19A23.51,23.51,0,0,0,236.8,188.09ZM120,104a8,8,0,0,1,16,0v40a8,8,0,0,1-16,0Zm8,88a12,12,0,1,1,12-12A12,12,0,0,1,128,192Z" />
                </svg>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
