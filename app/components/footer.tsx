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
