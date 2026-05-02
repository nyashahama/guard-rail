"use client";

export function Nav() {
  const calendarUrl = process.env.NEXT_PUBLIC_CALENDAR_URL || "#";

  return (
    <nav
      className="fixed left-0 right-0 top-0 z-50 flex h-[64px] items-center justify-between border-b border-white/[0.08] bg-background/80 px-4 backdrop-blur-md sm:h-[68px] sm:px-6 md:border-none md:bg-transparent md:backdrop-blur-none md:mix-blend-difference lg:px-12"
    >
      {/* Logo */}
      <a
        href="#"
        className="flex items-center gap-2 font-sans text-[12px] font-bold tracking-[0.15em] text-white uppercase no-underline sm:gap-2.5 sm:text-[13px]"
      >
        <div className="w-[26px] h-[26px] border-2 border-cyan flex items-center justify-center rounded-[4px]">
          <div className="w-2 h-2 bg-cyan" />
        </div>
        Guard Rail
      </a>

      {/* Right side */}
      <div className="flex min-w-0 items-center gap-3 sm:gap-4 lg:gap-8">
        <ul className="hidden list-none gap-5 lg:flex lg:gap-7">
          {[
            { label: "How It Works", href: "#how" },
            { label: "Features", href: "#features" },
            { label: "Pricing", href: "#pricing" },
            { label: "Docs", href: "/docs" },
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
          href={calendarUrl}
          className="border border-white/20 px-3 py-2 font-mono text-[10px] font-bold tracking-[0.1em] text-white/55 uppercase no-underline transition-all duration-300 hover:text-white sm:px-5 sm:py-2.5 sm:text-[11px]"
        >
          <span className="sm:hidden">Book</span>
          <span className="hidden sm:inline">Book Intro</span>
        </a>
      </div>
    </nav>
  );
}
