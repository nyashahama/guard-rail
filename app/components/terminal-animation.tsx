"use client";

import { useEffect, useState } from "react";

interface TerminalLine {
  text: string;
  color?: string;
  delay: number;
}

const SEQUENCE: TerminalLine[] = [
  { text: "$ incoming webhook POST /api/v1/transfer", color: "text-white/50", delay: 0 },
  { text: "", delay: 300 },
  { text: "{", color: "text-white/70", delay: 600 },
  { text: '  "amount": 250000,', color: "text-white/70", delay: 800 },
  { text: '  "recipient": "ZA-ACC-8812",', color: "text-white/70", delay: 1000 },
  { text: '  "callback": "https://evil.sh/exfil",', color: "text-crimson", delay: 1200 },
  { text: '  "memo": "Invoice #4401"', color: "text-white/70", delay: 1400 },
  { text: "}", color: "text-white/70", delay: 1600 },
  { text: "", delay: 1900 },
  { text: "[guardrail] Intercepting payload...", color: "text-cyan", delay: 2200 },
  { text: "[guardrail] Policy: egress-allowlist", color: "text-cyan", delay: 2800 },
  { text: '[guardrail] VIOLATION: "callback" domain not in allowlist', color: "text-crimson", delay: 3400 },
  { text: "", delay: 3800 },
  { text: "[result] BLOCKED — execution halted", color: "text-crimson font-bold", delay: 4200 },
  { text: "[audit]  Hash: 9f3a...c7e1 committed to ledger", color: "text-green", delay: 4800 },
  { text: "[audit]  Event logged: GR-EXE-449281", color: "text-green", delay: 5200 },
];

const CYCLE_DURATION = 8000;

export function TerminalAnimation() {
  const [visibleLines, setVisibleLines] = useState<number>(0);
  const [cycle, setCycle] = useState(0);

  useEffect(() => {
    setVisibleLines(0);

    const timers: ReturnType<typeof setTimeout>[] = [];

    SEQUENCE.forEach((line, i) => {
      timers.push(
        setTimeout(() => setVisibleLines(i + 1), line.delay)
      );
    });

    timers.push(
      setTimeout(() => {
        setCycle((c) => c + 1);
      }, CYCLE_DURATION)
    );

    return () => timers.forEach(clearTimeout);
  }, [cycle]);

  return (
    <div className="w-full max-w-xl bg-void rounded-xl border border-white/10 shadow-2xl overflow-hidden">
      {/* Title bar */}
      <div className="h-10 bg-surface border-b border-white/10 flex items-center px-4 gap-2">
        <div className="w-3 h-3 rounded-full bg-crimson/80" />
        <div className="w-3 h-3 rounded-full bg-yellow-500/80" />
        <div className="w-3 h-3 rounded-full bg-green/80" />
        <span className="ml-3 font-mono text-[11px] text-white/30">
          guardrail — interception log
        </span>
      </div>

      {/* Terminal body */}
      <div className="p-5 font-mono text-[13px] leading-relaxed h-80 overflow-hidden">
        {SEQUENCE.slice(0, visibleLines).map((line, i) => (
          <div key={`${cycle}-${i}`} className={`${line.color || "text-white/50"} min-h-[1.625rem]`}>
            {line.text}
          </div>
        ))}
        {visibleLines < SEQUENCE.length && (
          <span className="inline-block w-2 h-4 bg-cyan cursor-blink" />
        )}
      </div>
    </div>
  );
}
