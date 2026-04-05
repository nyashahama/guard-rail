"use client";

export function Nav() {
  return (
    <nav
      className="fixed top-0 left-0 right-0 z-50 flex items-center justify-between px-12 h-[68px] mix-blend-difference"
    >
      {/* Logo */}
      <a href="#" className="flex items-center gap-2.5 font-sans font-bold text-[13px] tracking-[0.15em] uppercase no-underline text-white">
        <div className="w-[26px] h-[26px] border-2 border-cyan flex items-center justify-center rounded-[4px]">
          <div className="w-2 h-2 bg-cyan" />
        </div>
        Guard Rail
      </a>

      {/* Right side */}
      <div className="flex items-center gap-8">
        <ul className="hidden md:flex gap-7 list-none">
          {[
            { label: "How It Works", href: "#how" },
            { label: "Features", href: "#features" },
            { label: "Pricing", href: "#pricing" },
          ].map((link) => (
            <li key={link.href}>
              <a
                href={link.href}
                className="font-mono text-[11px] tracking-[0.1em] uppercase text-white/50 no-underline hover:text-white transition-colors duration-200"
              >
                {link.label}
              </a>
            </li>
          ))}
        </ul>
        <a
          href="mailto:founders@guardrail.co.za"
          className="font-mono text-[11px] font-bold tracking-[0.1em] uppercase text-white border border-white/20 px-5 py-2.5 no-underline hover:bg-white hover:text-black transition-all duration-300"
        >
          Get Access
        </a>
      </div>
    </nav>
  );
}
