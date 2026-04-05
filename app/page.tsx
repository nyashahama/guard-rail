"use client";

import { useEffect, useRef } from "react";
import { Nav } from "./components/nav";
import { Hero } from "./components/hero";
import { Problem } from "./components/problem";
import { Capabilities } from "./components/capabilities";
import { Architecture } from "./components/architecture";
import { Trust } from "./components/trust";
import { Pricing } from "./components/pricing";
import { Footer } from "./components/footer";

export default function Home() {
  const scrollFillRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const onScroll = () => {
      if (!scrollFillRef.current) return;
      const pct =
        (window.scrollY / (document.body.scrollHeight - window.innerHeight)) * 100;
      scrollFillRef.current.style.height = Math.min(100, Math.max(10, pct)) + "%";
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <>
      {/* ── Fixed Ambient Background ── */}
      <div className="fixed inset-0 pointer-events-none z-0">
        <div className="bg-grid-pattern" />
        {/* Radial glow */}
        <div
          className="absolute inset-0"
          style={{
            background:
              "radial-gradient(circle at 50% 40%, rgba(0,240,255,0.15) 0%, transparent 60%)",
          }}
        />
        {/* Scanline effect */}
        <div className="scan-effect" />
      </div>

      {/* ── Scroll Indicator (left side) ── */}
      <aside
        className="fixed left-7 top-1/2 -translate-y-1/2 z-50 flex flex-col items-center gap-3 mix-blend-difference pointer-events-none"
      >
        <div className="relative w-px h-20 overflow-hidden" style={{ background: "rgba(255,255,255,0.15)" }}>
          <div
            ref={scrollFillRef}
            className="absolute top-0 left-0 right-0 bg-cyan transition-[height] duration-100 linear"
            style={{ height: "10%" }}
          />
        </div>
        <span
          className="font-mono text-[9px] tracking-[0.2em] uppercase text-white/40"
          style={{ writingMode: "vertical-lr", transform: "rotate(180deg)" }}
        >
          Scroll
        </span>
      </aside>

      {/* ── Corner Tag ── */}
      <div className="fixed right-7 bottom-6 z-50 font-mono text-[9px] tracking-[0.2em] uppercase text-white/25 mix-blend-difference">
        ZAF / Enterprise Auth
      </div>

      <Nav />

      <main className="relative z-[1]">
        <Hero />
        <hr className="border-none relative z-[1]" style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }} />
        <Problem />
        <hr className="border-none relative z-[1]" style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }} />
        <Capabilities />
        <hr className="border-none relative z-[1]" style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }} />
        <Architecture />
        <hr className="border-none relative z-[1]" style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }} />
        <Trust />
        <hr className="border-none relative z-[1]" style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }} />
        <Pricing />
      </main>

      <Footer />
    </>
  );
}
