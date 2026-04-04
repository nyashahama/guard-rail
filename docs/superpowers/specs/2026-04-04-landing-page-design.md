# Guard Rail — Public Landing Page Design Spec

## Overview

Replace the current investor pitch deck (`app/page.tsx`) with a public-facing landing page targeting enterprise prospects (primary) and investors/partners (secondary).

- **Status framing:** Pre-launch / early access
- **Primary CTA:** Request Demo / Request Early Access
- **Tone:** Technical & sharp (Vercel/Linear aesthetic)
- **Stack:** Next.js 16, React 19, Tailwind CSS 4 (existing)

## Design Language

- Dark theme carried forward but refined — less cyberpunk, more precision
- Remove scroll-snap deck architecture; replace with smooth long-scroll + scroll-triggered entrance animations
- Remove scanline effect, pulse rings, and "CONFIDENTIAL // SEED DECK" investor framing
- Keep cyan (`#00F0FF`) as primary accent, green (`#00FF55`) for success/safe states, crimson (`#FF2A4D`) for warnings
- Whitespace-forward layout with monospace accents for technical credibility
- Frosted glass panels used sparingly for elevated components
- Mobile-responsive throughout

## Sections

### 1. Navigation

Fixed top bar. Transparent on load, frosted glass (`backdrop-filter: blur`) on scroll.

- **Left:** Guard Rail logo (square-in-square icon + "GUARD RAIL" wordmark)
- **Right:** Text anchor links — "Product", "Architecture", "Pricing" + primary "Request Demo" button (solid cyan background, dark text)
- Mobile: hamburger menu

### 2. Hero

Full viewport height. Two-column on desktop (`lg` breakpoint), stacked on mobile.

**Left column:**
- Status badge: pill with pulse dot — "Now in early access"
- Headline: Single impactful sentence. Example direction: "The runtime that stands between your infrastructure and everything else." Large, bold, tight tracking.
- Sub-headline: 2 lines max. Plain technical language explaining what Guard Rail does. Example: "Sandbox every webhook, partner API call, and AI agent action before it touches your systems. Inspect, enforce, log — at the execution layer."
- Primary CTA: "Request Early Access" — solid button
- Secondary CTA: "See how it works" — text link with arrow icon, smooth-scrolls to Architecture section

**Right column:**
Animated terminal component showing a simulated interception sequence:
1. Incoming webhook payload arrives (JSON displayed)
2. Guard Rail intercepts — "Inspecting payload..."
3. Policy check — flags suspicious parameter
4. Result: "BLOCKED. Logged. Hash committed."

Typewriter-style animation, loops with a pause between cycles. Styled as a macOS-like terminal window (traffic light dots, monospace font, dark background with subtle border).

### 3. Problem

Visual separation via darker background.

- **Heading:** Single provocative line. Direction: "Your perimeter doesn't see what's running inside it."
- **Subheading:** One sentence elaborating — "Webhooks, no-code tools, partner APIs, and AI agents are executing logic against your systems with no inspection, no isolation, and no audit trail."
- **3-column card grid** (stacks on mobile), each card:
  1. **"Shadow workflows everywhere"** — Operations teams wiring Zapier/Make into production. IT finds out after the incident.
  2. **"AI agents with root-level access"** — LLMs calling internal tools with no validation of what they're requesting.
  3. **"Compliance gaps you can't see"** — Every unaudited webhook is a POPIA violation waiting to happen. Logs don't capture execution context.

Cards have a left accent border on hover (cyan). No investor-style statistics.

### 4. Capabilities (Product)

Bento grid layout — mix of large (2-col span) and standard cards.

**Heading:** "What Guard Rail does."

**Cards:**
1. **(Large) Sandboxed Runtime** — "Every external call executes in an ephemeral, isolated environment. Nothing touches your core until it's been inspected and cleared." Icon: cube.
2. **Policy Engine** — "Declarative rules that inspect at the payload level. Block by parameter, pattern, or business logic. YAML-configured, version-controlled." Icon: file-code.
3. **Deterministic Replay** — "Capture full execution state. When something fails or violates policy, replay the exact environment for debugging." Icon: replay/refresh.
4. **Cryptographic Audit Trail** — "Every execution produces a tamper-proof hash. Immutable logs built for compliance audits, not just debugging." Icon: database.
5. **Sub-millisecond Overhead** — "Guard Rail sits in the hot path. The architecture is designed for <1ms p99 added latency." Icon: lightning bolt.

Each card: icon + bold title + 1-2 sentence description. Glass panel style with cyan border on hover.

### 5. Architecture (Technical Deep-Dive)

**Heading:** "How it fits into your stack."

**Animated flow diagram:**
```
[Untrusted Trigger] ——> [Guard Rail] ——> [Enterprise Core]
```
- Three nodes connected by animated path lines
- On scroll into view, nodes light up sequentially left to right
- Guard Rail center node is larger, shows sub-steps: Inspect, Enforce, Log
- Untrusted Trigger labeled: "Webhook, Partner API, AI Agent"
- Enterprise Core labeled: "Internal DBs, Core Systems, ERPs"
- Color coding: white/neutral for trigger, cyan for Guard Rail, green for enterprise core

**Tabbed code block** below the diagram:
- **Tab 1 — Policy (YAML):** Sample policy blocking PII in outbound payloads
- **Tab 2 — Execution Log (JSON):** Sample logged execution event
- **Tab 3 — Integration (cURL):** 3-line example routing a webhook through Guard Rail

Terminal-window styling matching the hero animation.

### 6. Trust Signals

Compact centered section. Quiet confidence — no big headline.

**Row 1 — Logo bar:** Greyed-out placeholder logos with "Built for teams at" label. Placeholder-ready structure for when real logos are available.

**Row 2 — Compliance badges:** Horizontal row of small icon+label pairs:
- POPIA Compliant
- ZA Data Residency
- SOC 2 (placeholder)
- On-Prem / VPC Deploy

**Row 3 — Credibility line:** Single placeholder for either a customer quote or founder background statement.

Thin top/bottom borders for separation. No background change.

### 7. Pricing

**Heading:** "Simple, predictable pricing."
**Subheading:** Brief line about enterprise-aligned value.

**3-column card grid**, center card elevated:

**Pilot:**
- For teams evaluating Guard Rail
- 1 sandbox environment
- Up to 1M executions
- 14-day log retention
- Community support
- CTA: "Request Access" (outlined button)

**Enterprise (highlighted):**
- "Most Popular" badge
- Multi-tenant isolation
- Unlimited executions
- On-prem or VPC deployment
- Cryptographic audit SDK
- Dedicated support
- CTA: "Request Demo" (solid primary button)

**OEM:**
- Embed Guard Rail in your platform
- White-labeled runtime
- Custom SLAs
- Source code escrow
- CTA: "Talk to Us" (outlined button)

No prices displayed — pre-launch, CTAs drive conversations.

### 8. Footer

Full-width, darkest background shade.

**3-column grid:**
1. **Brand:** Logo + wordmark + one-line descriptor ("Secure execution for modern automation.")
2. **Navigation:** Product, Architecture, Pricing, Request Demo (anchor links)
3. **Company:** Cape Town, South Africa | Email contact | System Status (placeholder) | Social links (placeholder)

**Bottom bar:** `© 2025 Guard Rail Systems (Pty) Ltd.` | Privacy Policy | Terms of Service (placeholder links)

## What Gets Removed

- Scroll-snap deck architecture
- Scanline animation overlay
- "CONFIDENTIAL // SEED DECK v2.4" label
- Slide indicator sidebar
- "ZAF / Enterprise Auth" corner label
- Business model / pricing numbers (R95k/mo)
- Go-to-market phases (wedge strategy)
- Seed round CTA and data room link
- Traction metrics slide (3 pilots, 14M executions, etc.)
- Competitive comparison table
- Market timing / South Africa context slide
- System architecture terminal slide

## Technical Notes

- Single `page.tsx` file replacement — all sections in one page
- Keep inline SVG icon components (existing pattern), add new ones as needed
- Scroll-triggered animations via Intersection Observer (no external animation library)
- Smooth scroll behavior for anchor links
- Nav background transition on scroll via scroll event listener (existing pattern)
- Terminal animation via `useEffect` with `setTimeout`/`setInterval` sequencing
- All responsive via Tailwind breakpoints (`md`, `lg`)
- Existing `globals.css` theme variables preserved, scanline/deck-specific CSS removed
