export function Hero() {
  return (
    <section className="relative z-[1] min-h-screen flex items-center">
      <div
        className="grid gap-20 items-center pt-[100px] pb-20 px-20 w-full max-w-[1400px] mx-auto"
        style={{ gridTemplateColumns: "1fr 1fr" }}
      >
        {/* Left — Copy */}
        <div>
          {/* Badge */}
          <div className="inline-flex items-center gap-2 px-3.5 py-1.5 border border-cyan/30 bg-cyan/5 rounded-full mb-10 w-fit">
            <span className="w-1.5 h-1.5 bg-cyan rounded-full pulse-dot" style={{ boxShadow: "0 0 8px var(--cyan)" }} />
            <span className="font-mono text-[10px] tracking-[0.15em] uppercase text-cyan">
              Securing the next billion transactions
            </span>
          </div>

          <h1
            className="font-bold leading-[0.9] tracking-[-0.03em] mb-7"
            style={{ fontSize: "clamp(56px, 7vw, 96px)" }}
          >
            Secure <br />
            <span className="text-white/35">execution for</span>
            <br />
            modern <br />
            <span style={{ textShadow: "0 0 40px rgba(0,240,255,0.3)" }}>
              automation.
            </span>
          </h1>

          <p className="text-[19px] text-white/55 leading-[1.65] font-light max-w-[480px] mb-11">
            The enterprise-grade runtime engine. Sandboxing partner integrations,
            workflows, and AI agents for South Africa's critical financial
            infrastructure.
          </p>

          <div className="flex items-center gap-4">
            <a
              href="mailto:founders@guardrail.co.za?subject=Pilot Request"
              className="font-mono text-[12px] font-bold tracking-[0.1em] uppercase text-black bg-white px-7 py-3.5 no-underline inline-flex items-center gap-2.5 hover:bg-cyan transition-all duration-300"
              style={{ ["--tw-shadow" as string]: "none" }}
              onMouseEnter={(e) => {
                (e.currentTarget as HTMLElement).style.boxShadow =
                  "0 0 32px rgba(0,240,255,0.3)";
              }}
              onMouseLeave={(e) => {
                (e.currentTarget as HTMLElement).style.boxShadow = "none";
              }}
            >
              Start a Pilot
              <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 17L17 7M17 7H7M17 7v10" />
              </svg>
            </a>
            <a
              href="#how"
              className="font-mono text-[12px] tracking-[0.1em] uppercase text-white/50 border border-white/15 px-7 py-3.5 no-underline hover:text-white hover:border-white/40 transition-all duration-200"
            >
              See How It Works
            </a>
          </div>

          <div className="flex items-center gap-2 mt-15 font-mono text-[11px] text-white/35 tracking-[0.1em]">
            <span>Initialize</span>
            <svg className="bounce-y" width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 14l-7 7m0 0l-7-7m7 7V3" />
            </svg>
          </div>
        </div>

        {/* Right — Orbit Visual */}
        <div className="relative h-[560px] flex items-center justify-center">
          {/* Outer orbit ring */}
          <div
            className="orbit-spin absolute w-[380px] h-[380px] border border-white/8 rounded-full flex items-center justify-center"
          >
            <div className="w-[300px] h-[300px] border border-dashed border-cyan/18 rounded-full" />
            {/* Orbit dot */}
            <div
              className="absolute top-[-6px] left-1/2 -translate-x-1/2 w-3 h-3 bg-cyan rounded-full"
              style={{ boxShadow: "0 0 20px var(--cyan), 0 0 40px rgba(0,240,255,0.4)" }}
            />
          </div>

          {/* Center glow */}
          <div
            className="absolute w-40 h-40 bg-cyan/8 rounded-full"
            style={{ filter: "blur(40px)" }}
          />

          {/* Center box */}
          <div
            className="pulse-ring relative z-10 w-[120px] h-[120px] backdrop-blur-[12px] border border-white/10 rounded-[20px] flex flex-col items-center justify-center gap-2"
            style={{ background: "rgba(12,12,16,0.7)" }}
          >
            <svg width="36" height="36" fill="currentColor" viewBox="0 0 256 256" style={{ color: "var(--cyan)" }}>
              <path d="M208,40H48A16,16,0,0,0,32,56v56c0,52.72,25.52,84.67,46.93,102.19,23.06,18.86,46,26.61,47.17,27a8,8,0,0,0,5.34.05c1.12-.38,27.85-9.49,49.36-30.07C204.12,188.4,224,157.83,224,112V56A16,16,0,0,0,208,40Zm-34.34,77.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,156.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z" />
            </svg>
            <span className="font-mono text-[9px] font-bold tracking-[0.2em] uppercase text-cyan">Guard Rail</span>
          </div>

          {/* Float chip — top right */}
          <div
            className="absolute top-20 right-10 backdrop-blur-[12px] border border-white/8 px-3.5 py-2.5 rounded-[6px] flex items-center gap-2 font-mono text-[11px] whitespace-nowrap"
            style={{ background: "rgba(12,12,16,0.7)" }}
          >
            <span className="w-1.5 h-1.5 rounded-full flex-shrink-0 bg-green" style={{ boxShadow: "0 0 6px var(--green)" }} />
            <span className="text-green">Payload Signed</span>
          </div>

          {/* Float chip — bottom left */}
          <div
            className="absolute bottom-24 left-5 backdrop-blur-[12px] border border-white/8 px-3.5 py-2.5 rounded-[6px] flex items-center gap-2 font-mono text-[11px] whitespace-nowrap"
            style={{ background: "rgba(12,12,16,0.7)" }}
          >
            <span className="w-1.5 h-1.5 rounded-full flex-shrink-0 bg-cyan" style={{ boxShadow: "0 0 6px var(--cyan)" }} />
            <span className="text-white/60">0.04ms Latency</span>
          </div>
        </div>
      </div>
    </section>
  );
}
