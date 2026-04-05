export function Footer() {
  return (
    <footer
      className="relative z-[1] flex items-center justify-between px-20 py-10"
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

      <ul className="flex gap-6 list-none">
        {[
          { label: "Cape Town, ZAF", href: "#" },
          { label: "Privacy", href: "#" },
          { label: "System Status", href: "#" },
          { label: "Contact", href: "mailto:founders@guardrail.co.za" },
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

      <span className="font-mono text-[10px] text-white/20">
        © 2025 Guard Rail Systems (Pty) Ltd.
      </span>
    </footer>
  );
}
