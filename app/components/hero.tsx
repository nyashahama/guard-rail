import { TerminalAnimation } from "./terminal-animation";
import { ArrowRightIcon } from "./icons";

export function Hero() {
  return (
    <section className="min-h-screen flex items-center pt-24 pb-16 relative">
      <div className="max-w-7xl mx-auto px-6 lg:px-8 w-full">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-16 items-center">
          {/* Left — Copy */}
          <div>
            <div className="inline-flex items-center gap-2 px-3 py-1.5 border border-cyan/30 bg-cyan/5 rounded-full mb-8">
              <div className="w-1.5 h-1.5 bg-cyan rounded-full animate-pulse" />
              <span className="font-mono text-[11px] uppercase text-cyan tracking-wider">
                Now in early access
              </span>
            </div>

            <h1 className="text-5xl lg:text-7xl font-bold tracking-tight leading-[1.05] mb-6">
              The runtime that stands between your infrastructure{" "}
              <span className="text-white/40">and everything else.</span>
            </h1>

            <p className="text-lg lg:text-xl text-white/50 max-w-lg mb-10 leading-relaxed">
              Sandbox every webhook, partner API call, and AI agent action
              before it touches your systems. Inspect, enforce, log — at the
              execution layer.
            </p>

            <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
              <a
                href="#demo"
                className="px-8 py-3.5 bg-cyan text-black font-semibold rounded-lg hover:bg-white transition-colors text-sm"
              >
                Request Early Access
              </a>
              <a
                href="#architecture"
                className="flex items-center gap-2 text-sm text-white/50 hover:text-cyan transition-colors group"
              >
                See how it works
                <ArrowRightIcon className="w-4 h-4 group-hover:translate-x-0.5 transition-transform" />
              </a>
            </div>
          </div>

          {/* Right — Terminal */}
          <div className="hidden lg:flex justify-end">
            <TerminalAnimation />
          </div>
        </div>
      </div>
    </section>
  );
}
