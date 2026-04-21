export function Footer() {
  return (
    <footer
      className="relative z-[1] mx-auto flex w-full max-w-[1400px] flex-col items-start gap-6 px-6 py-8 sm:px-8 sm:py-10 lg:px-20 md:flex-row md:items-center md:justify-between"
      style={{ borderTop: "1px solid rgba(255,255,255,0.06)" }}
    >
      <a
        href="#"
        className="flex items-center gap-2.5 font-mono text-[12px] tracking-[0.12em] uppercase text-white/40 no-underline"
      >
        <div className="w-5 h-5 border-2 border-cyan flex items-center justify-center rounded-[4px]">
          <div className="w-1.5 h-1.5 bg-cyan" />
        </div>
        Guard Rail
      </a>

      <ul className="flex list-none flex-wrap gap-x-6 gap-y-3">
        {[
          { label: "Overview", href: "#" },
          { label: "Privacy", href: "#" },
          { label: "System Status", href: "#" },
          { label: "Contact", href: "mailto:nyashahama45@gmail.com" },
        ].map((link) => (
          <li key={link.label}>
            <a
              href={link.href}
              className="font-mono text-[11px] text-white/28 no-underline tracking-[0.08em] transition-colors duration-200 hover:text-cyan"
            >
              {link.label}
            </a>
          </li>
        ))}
      </ul>

      <span className="font-mono text-[10px] text-white/20 md:text-right">
        © 2025 Guard Rail Systems (Pty) Ltd.
      </span>
    </footer>
  );
}
