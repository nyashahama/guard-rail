# Landing Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the investor pitch deck with a public-facing landing page targeting enterprise prospects and investors.

**Architecture:** Single-page app split into focused component files under `app/components/`. Each section is its own component. A shared `useInView` hook powers scroll-triggered animations. The page root (`app/page.tsx`) composes all sections. `globals.css` is updated to remove deck-specific styles and add landing page utilities.

**Tech Stack:** Next.js 16, React 19, Tailwind CSS 4, TypeScript. No external dependencies.

---

## File Structure

```
app/
  page.tsx                    — Page root, composes all sections
  globals.css                 — Updated theme + landing page utilities
  components/
    icons.tsx                 — All SVG icon components (extracted from current page.tsx)
    nav.tsx                   — Fixed navigation bar
    hero.tsx                  — Hero section with animated terminal
    terminal-animation.tsx    — Terminal interception animation component
    problem.tsx               — Problem/pain section
    capabilities.tsx          — Product capabilities bento grid
    architecture.tsx          — Architecture flow diagram + tabbed code block
    trust.tsx                 — Trust signals section
    pricing.tsx               — Pricing tiers section
    footer.tsx                — Footer section
    use-in-view.ts            — Intersection Observer hook for scroll animations
```

---

### Task 1: Update globals.css

**Files:**
- Modify: `app/globals.css`

Remove deck-specific styles (scroll-snap, scanline, slide, pulse-element, map-node) and add landing page utilities (section spacing, scroll-triggered fade-in, glass panel refinement, smooth scroll).

- [ ] **Step 1: Replace globals.css with updated styles**

Replace the entire contents of `app/globals.css` with:

```css
@import "tailwindcss";

:root {
  --background: #030305;
  --foreground: #ffffff;
  --void: #030305;
  --surface: #0C0C10;
  --surface-light: #181820;
  --cyan: #00F0FF;
  --green: #00FF55;
  --crimson: #FF2A4D;
}

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-void: var(--void);
  --color-surface: var(--surface);
  --color-surface-light: var(--surface-light);
  --color-cyan: var(--cyan);
  --color-green: var(--green);
  --color-crimson: var(--crimson);
  --font-sans: var(--font-geist-sans);
  --font-mono: var(--font-geist-mono);
}

/* Smooth scroll for anchor links */
html {
  scroll-behavior: smooth;
}

/* Custom Scrollbar */
::-webkit-scrollbar {
  width: 6px;
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.1);
  border-radius: 3px;
}

/* Background Grid */
.bg-grid-pattern {
  background-image: linear-gradient(to right, rgba(255, 255, 255, 0.03) 1px, transparent 1px),
    linear-gradient(to bottom, rgba(255, 255, 255, 0.03) 1px, transparent 1px);
}

.bg-grid {
  background-size: 40px 40px;
  background-position: center center;
  mask-image: radial-gradient(ellipse at center, black 20%, transparent 80%);
  -webkit-mask-image: radial-gradient(ellipse at center, black 20%, transparent 80%);
}

/* Glass Panel */
.glass-panel {
  background: rgba(12, 12, 16, 0.6);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

/* Nav glass — only visible when scrolled */
.nav-glass {
  background: rgba(3, 3, 5, 0.8);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

/* Scroll-triggered fade-in */
.fade-in-section {
  opacity: 0;
  transform: translateY(24px);
  transition: opacity 0.7s ease-out, transform 0.7s ease-out;
}

.fade-in-section.is-visible {
  opacity: 1;
  transform: translateY(0);
}

/* Staggered children animation */
.stagger-children > * {
  opacity: 0;
  transform: translateY(16px);
  transition: opacity 0.5s ease-out, transform 0.5s ease-out;
}

.stagger-children.is-visible > *:nth-child(1) { transition-delay: 0ms; opacity: 1; transform: translateY(0); }
.stagger-children.is-visible > *:nth-child(2) { transition-delay: 100ms; opacity: 1; transform: translateY(0); }
.stagger-children.is-visible > *:nth-child(3) { transition-delay: 200ms; opacity: 1; transform: translateY(0); }
.stagger-children.is-visible > *:nth-child(4) { transition-delay: 300ms; opacity: 1; transform: translateY(0); }
.stagger-children.is-visible > *:nth-child(5) { transition-delay: 400ms; opacity: 1; transform: translateY(0); }

/* Terminal cursor blink */
@keyframes blink-cursor {
  0%, 100% { opacity: 1; }
  50% { opacity: 0; }
}

.cursor-blink {
  animation: blink-cursor 1s step-end infinite;
}

/* Architecture flow node glow */
@keyframes node-glow {
  0% { box-shadow: 0 0 0 rgba(0, 240, 255, 0); }
  50% { box-shadow: 0 0 30px rgba(0, 240, 255, 0.3); }
  100% { box-shadow: 0 0 0 rgba(0, 240, 255, 0); }
}

.node-active {
  animation: node-glow 2s ease-in-out infinite;
}

/* Flow line draw animation */
@keyframes draw-line {
  from { stroke-dashoffset: 100; }
  to { stroke-dashoffset: 0; }
}

.flow-line {
  stroke-dasharray: 100;
  stroke-dashoffset: 100;
}

.flow-line.is-visible {
  animation: draw-line 1s ease-out forwards;
}

body {
  background: var(--background);
  color: var(--foreground);
}
```

- [ ] **Step 2: Verify the build compiles**

Run: `cd /home/nyasha-hama/projects/guard-rail && npx next build 2>&1 | tail -20`
Expected: Build succeeds (the existing page.tsx still references old classes, but CSS changes are additive — old classes just become no-ops until page.tsx is replaced)

- [ ] **Step 3: Commit**

```bash
git add app/globals.css
git commit -m "update globals.css for landing page — remove deck styles, add scroll animations"
```

---

### Task 2: Extract Icon Components

**Files:**
- Create: `app/components/icons.tsx`

Extract all reusable SVG icon components from the current `page.tsx` and add new ones needed by the landing page.

- [ ] **Step 1: Create the icons file**

Create `app/components/icons.tsx`:

```tsx
export function ShieldCheckIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-12 h-12 ${className}`} fill="currentColor" viewBox="0 0 256 256">
      <path d="M208,40H48A16,16,0,0,0,32,56v56c0,52.72,25.52,84.67,46.93,102.19,23.06,18.86,46,26.61,47.17,27a8,8,0,0,0,5.34.05c1.12-.38,27.85-9.49,49.36-30.07C204.12,188.4,224,157.83,224,112V56A16,16,0,0,0,208,40Zm-34.34,77.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,156.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z" />
    </svg>
  );
}

export function CubeIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-8 h-8 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M21 16.5V7.8l-9-5.25L3 7.8v8.7l9 5.25 9-5.25zM3 7.8l9 5.25M12 13.05V22.5M21 7.8l-9 5.25" />
    </svg>
  );
}

export function FileCodeIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-8 h-8 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8l-6-6zM14 2v6h6M10 12l-2 2 2 2M14 12l2 2-2 2" />
    </svg>
  );
}

export function ReplayIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-8 h-8 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M1 4v6h6M23 20v-6h-6M20.49 9A9 9 0 005.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 013.51 15" />
    </svg>
  );
}

export function DatabaseIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-8 h-8 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <ellipse cx="12" cy="5" rx="9" ry="3" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" />
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5" />
    </svg>
  );
}

export function LightningIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-8 h-8 ${className}`} fill="currentColor" viewBox="0 0 24 24">
      <path d="M13 2L3 14h9l-1 10 10-12h-9l1-10z" />
    </svg>
  );
}

export function ServerIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-8 h-8 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <rect x="2" y="2" width="20" height="8" rx="2" ry="2" strokeWidth={1.5} />
      <rect x="2" y="14" width="20" height="8" rx="2" ry="2" strokeWidth={1.5} />
      <line x1="6" y1="6" x2="6.01" y2="6" strokeWidth={2} />
      <line x1="6" y1="18" x2="6.01" y2="18" strokeWidth={2} />
    </svg>
  );
}

export function ArrowRightIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-5 h-5 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 17L17 7M17 7H7M17 7v10" />
    </svg>
  );
}

export function CheckIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-4 h-4 shrink-0 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2.5} d="M5 13l4 4L19 7" />
    </svg>
  );
}

export function MenuIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-6 h-6 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 6h16M4 12h16M4 18h16" />
    </svg>
  );
}

export function CloseIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-6 h-6 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

export function ShieldSmallIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-5 h-5 ${className}`} fill="currentColor" viewBox="0 0 256 256">
      <path d="M208,40H48A16,16,0,0,0,32,56v56c0,52.72,25.52,84.67,46.93,102.19,23.06,18.86,46,26.61,47.17,27a8,8,0,0,0,5.34.05c1.12-.38,27.85-9.49,49.36-30.07C204.12,188.4,224,157.83,224,112V56A16,16,0,0,0,208,40Zm-34.34,77.66-56,56a8,8,0,0,1-11.32,0l-24-24a8,8,0,0,1,11.32-11.32L112,156.69l50.34-50.35a8,8,0,0,1,11.32,11.32Z" />
    </svg>
  );
}

export function GlobeIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-5 h-5 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <circle cx="12" cy="12" r="10" strokeWidth={1.5} />
      <path strokeLinecap="round" strokeWidth={1.5} d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" />
    </svg>
  );
}

export function LockIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-5 h-5 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <rect x="3" y="11" width="18" height="11" rx="2" ry="2" strokeWidth={1.5} />
      <path strokeWidth={1.5} d="M7 11V7a5 5 0 0110 0v4" />
    </svg>
  );
}

export function CloudIcon({ className = "" }: { className?: string }) {
  return (
    <svg className={`w-5 h-5 ${className}`} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M18 10h-1.26A8 8 0 109 20h9a5 5 0 000-10z" />
    </svg>
  );
}
```

- [ ] **Step 2: Verify no TypeScript errors**

Run: `cd /home/nyasha-hama/projects/guard-rail && npx tsc --noEmit 2>&1 | tail -10`
Expected: No errors from icons.tsx (page.tsx may still error since it hasn't been updated yet — that's fine)

- [ ] **Step 3: Commit**

```bash
git add app/components/icons.tsx
git commit -m "extract SVG icon components into app/components/icons.tsx"
```

---

### Task 3: Create useInView Hook

**Files:**
- Create: `app/components/use-in-view.ts`

A reusable Intersection Observer hook for scroll-triggered animations.

- [ ] **Step 1: Create the hook**

Create `app/components/use-in-view.ts`:

```ts
"use client";

import { useEffect, useRef, useState } from "react";

export function useInView(threshold = 0.15) {
  const ref = useRef<HTMLDivElement>(null);
  const [isVisible, setIsVisible] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setIsVisible(true);
          observer.unobserve(el);
        }
      },
      { threshold }
    );

    observer.observe(el);
    return () => observer.disconnect();
  }, [threshold]);

  return { ref, isVisible };
}
```

- [ ] **Step 2: Commit**

```bash
git add app/components/use-in-view.ts
git commit -m "add useInView hook for scroll-triggered animations"
```

---

### Task 4: Build Navigation Component

**Files:**
- Create: `app/components/nav.tsx`

Fixed top bar with transparent-to-glass transition on scroll. Mobile hamburger menu.

- [ ] **Step 1: Create the nav component**

Create `app/components/nav.tsx`:

```tsx
"use client";

import { useEffect, useState } from "react";
import { MenuIcon, CloseIcon } from "./icons";

const NAV_LINKS = [
  { label: "Product", href: "#product" },
  { label: "Architecture", href: "#architecture" },
  { label: "Pricing", href: "#pricing" },
];

export function Nav() {
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 32);
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <nav
      className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
        scrolled ? "nav-glass" : "bg-transparent"
      }`}
    >
      <div className="max-w-7xl mx-auto px-6 lg:px-8 flex items-center justify-between h-16">
        {/* Logo */}
        <a href="#" className="flex items-center gap-3">
          <div className="w-6 h-6 border-2 border-cyan flex items-center justify-center rounded-sm">
            <div className="w-2 h-2 bg-cyan" />
          </div>
          <span className="font-bold tracking-widest uppercase text-sm">
            Guard Rail
          </span>
        </a>

        {/* Desktop links */}
        <div className="hidden md:flex items-center gap-8">
          {NAV_LINKS.map((link) => (
            <a
              key={link.href}
              href={link.href}
              className="text-sm text-white/60 hover:text-white transition-colors"
            >
              {link.label}
            </a>
          ))}
          <a
            href="#demo"
            className="text-sm font-semibold bg-cyan text-black px-5 py-2 rounded-lg hover:bg-white transition-colors"
          >
            Request Demo
          </a>
        </div>

        {/* Mobile hamburger */}
        <button
          className="md:hidden text-white/60 hover:text-white"
          onClick={() => setMobileOpen(!mobileOpen)}
          aria-label="Toggle menu"
        >
          {mobileOpen ? <CloseIcon /> : <MenuIcon />}
        </button>
      </div>

      {/* Mobile menu */}
      {mobileOpen && (
        <div className="md:hidden nav-glass border-t border-white/5 px-6 py-6 flex flex-col gap-4">
          {NAV_LINKS.map((link) => (
            <a
              key={link.href}
              href={link.href}
              className="text-sm text-white/60 hover:text-white transition-colors"
              onClick={() => setMobileOpen(false)}
            >
              {link.label}
            </a>
          ))}
          <a
            href="#demo"
            className="text-sm font-semibold bg-cyan text-black px-5 py-2 rounded-lg hover:bg-white transition-colors text-center"
            onClick={() => setMobileOpen(false)}
          >
            Request Demo
          </a>
        </div>
      )}
    </nav>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add app/components/nav.tsx
git commit -m "add Nav component with scroll glass effect and mobile menu"
```

---

### Task 5: Build Terminal Animation Component

**Files:**
- Create: `app/components/terminal-animation.tsx`

The animated terminal that shows a payload interception sequence. Used in the Hero section.

- [ ] **Step 1: Create the terminal animation**

Create `app/components/terminal-animation.tsx`:

```tsx
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
```

- [ ] **Step 2: Commit**

```bash
git add app/components/terminal-animation.tsx
git commit -m "add TerminalAnimation component with typewriter interception sequence"
```

---

### Task 6: Build Hero Section

**Files:**
- Create: `app/components/hero.tsx`

Full-height hero with headline, CTAs, and the terminal animation.

- [ ] **Step 1: Create the hero component**

Create `app/components/hero.tsx`:

```tsx
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
```

- [ ] **Step 2: Commit**

```bash
git add app/components/hero.tsx
git commit -m "add Hero section with headline, CTAs, and terminal animation"
```

---

### Task 7: Build Problem Section

**Files:**
- Create: `app/components/problem.tsx`

Three pain-point cards on a darker background.

- [ ] **Step 1: Create the problem component**

Create `app/components/problem.tsx`:

```tsx
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
```

- [ ] **Step 2: Commit**

```bash
git add app/components/problem.tsx
git commit -m "add Problem section with pain point cards"
```

---

### Task 8: Build Capabilities Section

**Files:**
- Create: `app/components/capabilities.tsx`

Bento grid with 5 product capability cards.

- [ ] **Step 1: Create the capabilities component**

Create `app/components/capabilities.tsx`:

```tsx
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
```

- [ ] **Step 2: Commit**

```bash
git add app/components/capabilities.tsx
git commit -m "add Capabilities section with bento grid layout"
```

---

### Task 9: Build Architecture Section

**Files:**
- Create: `app/components/architecture.tsx`

Animated flow diagram + tabbed code block.

- [ ] **Step 1: Create the architecture component**

Create `app/components/architecture.tsx`:

```tsx
"use client";

import { useState } from "react";
import { useInView } from "./use-in-view";
import { LightningIcon, ShieldSmallIcon, ServerIcon } from "./icons";

const TABS = [
  {
    label: "Policy (YAML)",
    content: `# guardrail-policy.yaml
rules:
  - name: block-external-callbacks
    trigger: webhook.inbound
    match:
      field: callback
      condition: domain_not_in
      values:
        - "*.guardrail.co.za"
        - "*.internal.bank.za"
    action: block
    severity: critical
    log: true`,
  },
  {
    label: "Execution Log (JSON)",
    content: `{
  "event_id": "GR-EXE-449281",
  "timestamp": "2026-04-04T14:32:01.004Z",
  "trigger": "webhook.inbound",
  "source_ip": "41.13.22.91",
  "policy": "block-external-callbacks",
  "verdict": "BLOCKED",
  "violation_field": "callback",
  "violation_value": "https://evil.sh/exfil",
  "hash": "9f3a8b...c7e1",
  "latency_ms": 0.42
}`,
  },
  {
    label: "Integration (cURL)",
    content: `# Route any webhook through Guard Rail
curl -X POST https://gw.guardrail.co.za/v1/execute \\
  -H "Authorization: Bearer gr_live_sk_..." \\
  -H "Content-Type: application/json" \\
  -d @payload.json`,
  },
];

export function Architecture() {
  const { ref, isVisible } = useInView(0.1);
  const [activeTab, setActiveTab] = useState(0);

  return (
    <section id="architecture" className="py-32 bg-surface-light">
      <div className="max-w-7xl mx-auto px-6 lg:px-8">
        <div
          ref={ref}
          className={`fade-in-section ${isVisible ? "is-visible" : ""}`}
        >
          <h2 className="text-4xl lg:text-5xl font-bold tracking-tight mb-20 text-center">
            How it fits into your stack.
          </h2>

          {/* Flow Diagram */}
          <div className="flex flex-col md:flex-row items-center justify-center gap-6 lg:gap-12 mb-24">
            {/* Untrusted Trigger */}
            <div
              className={`flex flex-col items-center transition-all duration-700 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"
              }`}
            >
              <div className="w-20 h-20 rounded-full border border-white/20 bg-surface flex items-center justify-center mb-3">
                <LightningIcon className="text-white/60 w-6 h-6" />
              </div>
              <h4 className="font-bold text-sm mb-1">Untrusted Trigger</h4>
              <span className="font-mono text-[11px] text-white/40 text-center">
                Webhook, Partner API,
                <br />
                AI Agent
              </span>
            </div>

            {/* Arrow 1 */}
            <svg
              className={`hidden md:block w-24 h-4 transition-all duration-700 delay-300 ${
                isVisible ? "opacity-100" : "opacity-0"
              }`}
              viewBox="0 0 96 16"
            >
              <line
                x1="0" y1="8" x2="80" y2="8"
                stroke="rgba(255,255,255,0.2)"
                strokeWidth="1.5"
                className={`flow-line ${isVisible ? "is-visible" : ""}`}
                style={{ animationDelay: "0.4s" }}
              />
              <polygon points="80,3 92,8 80,13" fill="#00F0FF" className={`transition-opacity duration-300 ${isVisible ? "opacity-100" : "opacity-0"}`} style={{ transitionDelay: "1s" }} />
            </svg>

            {/* Guard Rail (center) */}
            <div
              className={`flex flex-col items-center transition-all duration-700 delay-500 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"
              }`}
            >
              <div className={`w-28 h-28 rounded-2xl glass-panel border-cyan/50 flex flex-col items-center justify-center mb-3 ${isVisible ? "node-active" : ""}`}>
                <ShieldSmallIcon className="text-cyan w-8 h-8" />
                <span className="font-mono text-[9px] font-bold tracking-widest uppercase mt-1.5">
                  Guard Rail
                </span>
              </div>
              <div className="flex gap-2 font-mono text-[10px]">
                <span className="px-2 py-1 bg-surface border border-white/10 rounded">
                  Inspect
                </span>
                <span className="px-2 py-1 bg-surface border border-white/10 rounded">
                  Enforce
                </span>
                <span className="px-2 py-1 bg-surface border border-white/10 rounded">
                  Log
                </span>
              </div>
            </div>

            {/* Arrow 2 */}
            <svg
              className={`hidden md:block w-24 h-4 transition-all duration-700 delay-700 ${
                isVisible ? "opacity-100" : "opacity-0"
              }`}
              viewBox="0 0 96 16"
            >
              <line
                x1="0" y1="8" x2="80" y2="8"
                stroke="rgba(255,255,255,0.2)"
                strokeWidth="1.5"
                className={`flow-line ${isVisible ? "is-visible" : ""}`}
                style={{ animationDelay: "0.8s" }}
              />
              <polygon points="80,3 92,8 80,13" fill="#00FF55" className={`transition-opacity duration-300 ${isVisible ? "opacity-100" : "opacity-0"}`} style={{ transitionDelay: "1.4s" }} />
            </svg>

            {/* Enterprise Core */}
            <div
              className={`flex flex-col items-center transition-all duration-700 delay-1000 ${
                isVisible ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"
              }`}
            >
              <div className="w-20 h-20 rounded-lg border border-green/30 bg-green/5 flex items-center justify-center mb-3">
                <ServerIcon className="text-green w-6 h-6" />
              </div>
              <h4 className="font-bold text-sm mb-1">Enterprise Core</h4>
              <span className="font-mono text-[11px] text-white/40 text-center">
                Internal DBs, Core Systems,
                <br />
                ERPs
              </span>
            </div>
          </div>

          {/* Tabbed Code Block */}
          <div className="max-w-3xl mx-auto">
            <div className="bg-void rounded-xl border border-white/10 shadow-2xl overflow-hidden">
              {/* Tab bar */}
              <div className="flex border-b border-white/10">
                {TABS.map((tab, i) => (
                  <button
                    key={tab.label}
                    onClick={() => setActiveTab(i)}
                    className={`px-5 py-3 font-mono text-xs transition-colors cursor-pointer ${
                      activeTab === i
                        ? "text-cyan border-b-2 border-cyan bg-white/5"
                        : "text-white/40 hover:text-white/60"
                    }`}
                  >
                    {tab.label}
                  </button>
                ))}
              </div>

              {/* Code content */}
              <pre className="p-6 font-mono text-[13px] leading-relaxed text-white/70 overflow-x-auto">
                {TABS[activeTab].content}
              </pre>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add app/components/architecture.tsx
git commit -m "add Architecture section with animated flow diagram and tabbed code block"
```

---

### Task 10: Build Trust Signals Section

**Files:**
- Create: `app/components/trust.tsx`

Logo placeholders, compliance badges, credibility line.

- [ ] **Step 1: Create the trust component**

Create `app/components/trust.tsx`:

```tsx
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
```

- [ ] **Step 2: Commit**

```bash
git add app/components/trust.tsx
git commit -m "add Trust section with placeholder logos, compliance badges, credibility line"
```

---

### Task 11: Build Pricing Section

**Files:**
- Create: `app/components/pricing.tsx`

Three-tier pricing cards, center card highlighted.

- [ ] **Step 1: Create the pricing component**

Create `app/components/pricing.tsx`:

```tsx
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
```

- [ ] **Step 2: Commit**

```bash
git add app/components/pricing.tsx
git commit -m "add Pricing section with three-tier card layout"
```

---

### Task 12: Build Footer

**Files:**
- Create: `app/components/footer.tsx`

Three-column grid + bottom bar.

- [ ] **Step 1: Create the footer component**

Create `app/components/footer.tsx`:

```tsx
export function Footer() {
  return (
    <footer id="demo" className="bg-void border-t border-white/5 pt-16 pb-8">
      <div className="max-w-7xl mx-auto px-6 lg:px-8">
        {/* Demo CTA banner */}
        <div className="text-center mb-20">
          <h2 className="text-3xl lg:text-4xl font-bold tracking-tight mb-4">
            Ready to secure your automation layer?
          </h2>
          <p className="text-white/50 mb-8 max-w-md mx-auto">
            Get early access to Guard Rail. Talk to our engineering team
            about your infrastructure.
          </p>
          <a
            href="mailto:founders@guardrail.co.za?subject=Demo%20Request"
            className="inline-block px-10 py-4 bg-cyan text-black font-semibold rounded-lg hover:bg-white transition-colors"
          >
            Request a Demo
          </a>
        </div>

        {/* Footer grid */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-12 pb-12 border-b border-white/5">
          {/* Brand */}
          <div>
            <div className="flex items-center gap-3 mb-4">
              <div className="w-5 h-5 border-2 border-cyan flex items-center justify-center rounded-sm">
                <div className="w-1.5 h-1.5 bg-cyan" />
              </div>
              <span className="font-bold tracking-widest uppercase text-sm">
                Guard Rail
              </span>
            </div>
            <p className="text-sm text-white/40">
              Secure execution for modern automation.
            </p>
          </div>

          {/* Navigation */}
          <div>
            <h4 className="font-mono text-xs text-white/30 uppercase tracking-wider mb-4">
              Navigation
            </h4>
            <ul className="space-y-3">
              {[
                { label: "Product", href: "#product" },
                { label: "Architecture", href: "#architecture" },
                { label: "Pricing", href: "#pricing" },
                { label: "Request Demo", href: "#demo" },
              ].map((link) => (
                <li key={link.label}>
                  <a
                    href={link.href}
                    className="text-sm text-white/50 hover:text-white transition-colors"
                  >
                    {link.label}
                  </a>
                </li>
              ))}
            </ul>
          </div>

          {/* Company */}
          <div>
            <h4 className="font-mono text-xs text-white/30 uppercase tracking-wider mb-4">
              Company
            </h4>
            <ul className="space-y-3 text-sm text-white/50">
              <li>Cape Town, South Africa</li>
              <li>
                <a
                  href="mailto:founders@guardrail.co.za"
                  className="hover:text-white transition-colors"
                >
                  founders@guardrail.co.za
                </a>
              </li>
              <li>
                <a href="#" className="hover:text-white transition-colors">
                  System Status
                </a>
              </li>
            </ul>
          </div>
        </div>

        {/* Bottom bar */}
        <div className="flex flex-col md:flex-row justify-between items-center pt-8 text-xs font-mono text-white/30">
          <div>&copy; 2025 Guard Rail Systems (Pty) Ltd.</div>
          <div className="flex gap-6 mt-4 md:mt-0">
            <a href="#" className="hover:text-white transition-colors">
              Privacy Policy
            </a>
            <a href="#" className="hover:text-white transition-colors">
              Terms of Service
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add app/components/footer.tsx
git commit -m "add Footer with demo CTA, nav links, and company info"
```

---

### Task 13: Assemble Page Root

**Files:**
- Modify: `app/page.tsx` (full replacement)

Compose all section components into the final landing page.

- [ ] **Step 1: Replace page.tsx**

Replace the entire contents of `app/page.tsx` with:

```tsx
import { Nav } from "./components/nav";
import { Hero } from "./components/hero";
import { Problem } from "./components/problem";
import { Capabilities } from "./components/capabilities";
import { Architecture } from "./components/architecture";
import { Trust } from "./components/trust";
import { Pricing } from "./components/pricing";
import { Footer } from "./components/footer";

export default function Home() {
  return (
    <>
      {/* Ambient Background */}
      <div className="fixed inset-0 pointer-events-none z-0">
        <div className="absolute inset-0 bg-grid-pattern bg-grid opacity-20" />
      </div>

      <Nav />

      <main className="relative z-10">
        <Hero />
        <Problem />
        <Capabilities />
        <Architecture />
        <Trust />
        <Pricing />
      </main>

      <Footer />
    </>
  );
}
```

- [ ] **Step 2: Verify the build compiles**

Run: `cd /home/nyasha-hama/projects/guard-rail && npx next build 2>&1 | tail -20`
Expected: Build succeeds with no errors

- [ ] **Step 3: Verify the dev server renders**

Run: `cd /home/nyasha-hama/projects/guard-rail && npx next dev --port 3099 &` then `sleep 3 && curl -s http://localhost:3099 | head -50`
Expected: HTML output containing "Guard Rail" content. Kill the dev server after verification.

- [ ] **Step 4: Commit**

```bash
git add app/page.tsx
git commit -m "replace pitch deck with landing page — assemble all sections"
```

---

### Task 14: Visual QA and Polish

**Files:**
- May modify: any component file for spacing, responsive, or visual fixes

- [ ] **Step 1: Start dev server and check all sections render**

Run: `cd /home/nyasha-hama/projects/guard-rail && npx next dev --port 3099`
Open `http://localhost:3099` in browser. Walk through each section visually:
- Nav: transparent → glass on scroll, mobile hamburger works
- Hero: headline readable, terminal animation plays, CTAs visible
- Problem: cards render in 3-col on desktop, stack on mobile
- Capabilities: bento grid lays out correctly
- Architecture: flow diagram centers, tabs switch, code renders
- Trust: badges centered, placeholder logos visible
- Pricing: 3 cards, center elevated, CTAs work
- Footer: 3-col grid, bottom bar, demo CTA

- [ ] **Step 2: Fix any issues found**

Address spacing, overflow, or responsive issues. Each fix is a targeted edit to the relevant component.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "visual QA polish — spacing and responsive fixes"
```
