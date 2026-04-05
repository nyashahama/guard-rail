"use client";

export function Architecture() {
  const conditions = [
    "domain_not_in",
    "domain_in",
    "regex_match",
    "regex_not_match",
    "equals",
    "not_equals",
    "contains",
    "not_contains",
    "size_exceeds",
    "field_exists",
    "field_not_exists",
  ];

  return (
    <section
      className="relative z-[1]"
      style={{
        background: "var(--surface-light)",
        borderTop: "1px solid rgba(255,255,255,0.04)",
        borderBottom: "1px solid rgba(255,255,255,0.04)",
      }}
    >
      <div className="px-20 py-[120px] max-w-[1400px] mx-auto">
        <div className="grid gap-16 items-start" style={{ gridTemplateColumns: "1fr 1fr" }}>
          {/* Left */}
          <div>
            <span className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan block mb-4">
              // Policy Engine
            </span>
            <h2
              className="font-extrabold tracking-[-0.025em] leading-[1.0]"
              style={{ fontSize: "clamp(40px, 5vw, 72px)" }}
            >
              YAML policies.
              <br />
              <span className="text-white/28">
                Version-controlled.
                <br />
                Hot-reloaded.
              </span>
            </h2>
            <p className="text-[17px] text-white/55 leading-[1.65] font-light mt-6">
              Define security rules as YAML files alongside your infrastructure
              config. Guard Rail watches for changes and reloads without downtime.
              Bad syntax keeps the previous valid set active.
            </p>
            {/* Condition tags */}
            <div className="flex flex-wrap gap-2 mt-6">
              {conditions.map((c) => (
                <span
                  key={c}
                  className="font-mono text-[11px] px-3 py-[5px] border border-white/10 text-white/45 tracking-[0.05em] cursor-default transition-colors duration-200 hover:border-cyan/40 hover:text-cyan"
                >
                  {c}
                </span>
              ))}
            </div>
          </div>

          {/* Right — YAML Terminal */}
          <div
            className="relative overflow-hidden border border-white/10"
            style={{
              background: "var(--surface)",
            }}
          >
            {/* Gradient overlay */}
            <div
              className="absolute inset-0 pointer-events-none"
              style={{
                background: "linear-gradient(135deg, rgba(0,240,255,0.04) 0%, transparent 60%)",
              }}
            />
            {/* Terminal bar */}
            <div
              className="flex items-center justify-between px-4 py-3 border-b border-white/6"
              style={{ background: "var(--surface-light)" }}
            >
              <div className="flex gap-1.5">
                <span className="w-2.5 h-2.5 rounded-full bg-crimson" />
                <span className="w-2.5 h-2.5 rounded-full bg-[#FFB300]" />
                <span className="w-2.5 h-2.5 rounded-full bg-green" />
              </div>
              <span className="font-mono text-[10px] tracking-[0.12em] uppercase text-white/30">
                policies/security.yaml
              </span>
              <div />
            </div>
            {/* Terminal body */}
            <div className="px-7 py-7 font-mono text-[12.5px] leading-[1.85] overflow-x-auto">
              <span className="text-white/20"># Block payloads with external callback URLs</span>
              {"\n"}
              <span className="text-cyan">policies</span>:{"\n"}
              <span className="block pl-4">
                - <span className="text-cyan">name</span>:{" "}
                <span className="text-green">block-external-callbacks</span>
              </span>
              <span className="block pl-8">
                <span className="text-cyan">rules</span>:
              </span>
              <span className="block pl-12">
                - <span className="text-cyan">field</span>:{" "}
                <span className="text-green">&quot;$.callback&quot;</span>
              </span>
              <span className="block pl-12">
                {"  "}
                <span className="text-cyan">condition</span>:{" "}
                <span className="text-[#CF8DFB]">domain_not_in</span>
              </span>
              <span className="block pl-12">
                {"  "}
                <span className="text-cyan">values</span>: [
                <span className="text-green">&quot;*.internal.bank.za&quot;</span>]
              </span>
              <span className="block pl-12">
                {"  "}
                <span className="text-cyan">action</span>:{" "}
                <span className="text-[#FFD580]">block</span> ·{" "}
                <span className="text-cyan">severity</span>:{" "}
                <span className="text-[#FFD580]">critical</span>
              </span>
              {"\n"}
              <span className="block pl-4">
                - <span className="text-cyan">name</span>:{" "}
                <span className="text-green">pii-detection</span>
              </span>
              <span className="block pl-8">
                <span className="text-cyan">description</span>: Block SA ID numbers in payload
              </span>
              <span className="block pl-8">
                <span className="text-cyan">rules</span>:
              </span>
              <span className="block pl-12">
                - <span className="text-cyan">field</span>:{" "}
                <span className="text-green">&quot;$..**&quot;</span>
              </span>
              <span className="block pl-12">
                {"  "}
                <span className="text-cyan">condition</span>:{" "}
                <span className="text-[#CF8DFB]">regex_match</span>
              </span>
              <span className="block pl-12">
                {"  "}
                <span className="text-cyan">pattern</span>:{" "}
                <span className="text-green">&quot;\b\d&#123;13&#125;\b&quot;</span>
              </span>
              <span className="block pl-12">
                {"  "}
                <span className="text-cyan">action</span>:{" "}
                <span className="text-[#FFD580]">block</span> ·{" "}
                <span className="text-cyan">severity</span>:{" "}
                <span className="text-[#FFD580]">critical</span>
              </span>
              {"\n"}
              <span className="block pl-4">
                - <span className="text-cyan">name</span>:{" "}
                <span className="text-green">payload-size-limit</span>
              </span>
              <span className="block pl-8">
                <span className="text-cyan">rules</span>:
              </span>
              <span className="block pl-12">
                - <span className="text-cyan">field</span>:{" "}
                <span className="text-green">&quot;$&quot;</span> ·{" "}
                <span className="text-cyan">condition</span>:{" "}
                <span className="text-[#CF8DFB]">size_exceeds</span>
              </span>
              <span className="block pl-12">
                {"  "}
                <span className="text-cyan">max_bytes</span>:{" "}
                <span className="text-[#FFD580]">102400</span> ·{" "}
                <span className="text-cyan">action</span>:{" "}
                <span className="text-[#FFD580]">block</span>
              </span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
