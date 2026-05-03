import { notFound } from "next/navigation";
import Link from "next/link";
import { CSSProperties } from "react";
import { docsManifest, resolveDocBySlug, resolveDocFile } from "../_catalog";
import fs from "node:fs/promises";
import path from "node:path";

function toSafeLines(raw: string) {
  return raw.replace(/\r\n/g, "\n");
}

function docTypeLabel(kind: string) {
  return kind === "policy" ? "Policy Template" : "Onboarding Doc";
}

export async function generateStaticParams() {
  return docsManifest.map((doc) => ({ slug: doc.slug }));
}

export default async function DocsDocumentPage({
  params,
}: {
  params: Promise<{ slug: string }> | { slug: string };
}) {
  const { slug } = await Promise.resolve(params as { slug: string });
  const doc = resolveDocBySlug(slug);

  if (!doc) {
    notFound();
  }

  const filePath = resolveDocFile(slug);
  if (!filePath) {
    notFound();
  }

  const absolutePath = path.resolve(filePath);
  let content = "";

  try {
    content = await fs.readFile(absolutePath, "utf8");
  } catch (error) {
    console.error("Could not read docs file", absolutePath, error);
    notFound();
  }

  const normalized = toSafeLines(content);
  const isYaml = absolutePath.endsWith(".yaml") || absolutePath.endsWith(".yml");

  const preStyle: CSSProperties = isYaml
    ? {
        whiteSpace: "pre",
        fontFamily: "var(--font-geist-mono)",
      }
    : {
        whiteSpace: "pre-wrap",
        fontFamily: "var(--font-geist-mono)",
      };

  return (
    <main className="relative z-[1]">
      <div className="fixed inset-0 pointer-events-none z-0">
        <div className="bg-grid-pattern" />
      </div>
      <div className="relative z-[1] max-w-[1100px] mx-auto px-6 sm:px-8 lg:px-12 py-[90px]">
        <p className="font-mono text-[11px] tracking-[0.25em] uppercase text-cyan mb-4">
          {docTypeLabel(doc.kind)}
        </p>
        <h1 className="font-extrabold tracking-[-0.025em] leading-none text-[38px] sm:text-[54px] md:text-[64px] max-w-[20ch]">
          {doc.title}
        </h1>
        <p className="mt-4 font-mono text-[13px] text-white/55 max-w-[80ch] leading-[1.7]">
          {doc.description}
        </p>

        <Link
          href="/docs"
          className="mt-8 inline-flex font-mono text-[11px] tracking-[0.1em] uppercase text-white/55 border border-white/15 px-4 py-2.5 hover:text-cyan hover:border-cyan/60 transition-colors duration-200"
        >
          Back to onboarding docs
        </Link>

        <section className="mt-8 border border-white/15 bg-surface p-6 sm:p-8">
          <p className="font-mono text-[11px] tracking-[0.12em] uppercase text-white/35 mb-4">
            Source file: {path.relative(process.cwd(), absolutePath)}
          </p>
          <pre
            className="rounded-[4px] border border-white/10 p-5 text-[12px] leading-[1.7] text-white/70"
            style={preStyle}
          >
            {normalized}
          </pre>
        </section>
      </div>
    </main>
  );
}
