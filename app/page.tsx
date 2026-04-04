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
