"use client";

import { FormEvent, useState } from "react";

const routeOptions = ["1 route", "2 routes", "3+ routes"];

export function LeadForm() {
  const [status, setStatus] = useState<"idle" | "submitting" | "sent" | "error">("idle");

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setStatus("submitting");

    const formElement = event.currentTarget;
    const form = new FormData(formElement);
    try {
      const response = await fetch("/api/leads", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          name: form.get("name"),
          email: form.get("email"),
          company: form.get("company"),
          role: form.get("role"),
          routeCount: form.get("routeCount"),
          useCase: form.get("useCase"),
        }),
      });

      setStatus(response.ok ? "sent" : "error");
      if (response.ok) {
        formElement.reset();
      }
    } catch {
      setStatus("error");
    }
  }

  return (
    <form id="pilot-contact" onSubmit={onSubmit} className="grid gap-4">
      <div className="grid gap-4 md:grid-cols-2">
        <label className="grid gap-2 font-mono text-[11px] uppercase tracking-[0.1em] text-white/45">
          Name
          <input
            name="name"
            required
            maxLength={120}
            className="border border-white/15 bg-black px-4 py-3 text-[14px] normal-case tracking-normal text-white outline-none focus:border-cyan"
          />
        </label>
        <label className="grid gap-2 font-mono text-[11px] uppercase tracking-[0.1em] text-white/45">
          Work Email
          <input
            name="email"
            type="email"
            required
            maxLength={180}
            className="border border-white/15 bg-black px-4 py-3 text-[14px] normal-case tracking-normal text-white outline-none focus:border-cyan"
          />
        </label>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <label className="grid gap-2 font-mono text-[11px] uppercase tracking-[0.1em] text-white/45">
          Company
          <input
            name="company"
            required
            maxLength={180}
            className="border border-white/15 bg-black px-4 py-3 text-[14px] normal-case tracking-normal text-white outline-none focus:border-cyan"
          />
        </label>
        <label className="grid gap-2 font-mono text-[11px] uppercase tracking-[0.1em] text-white/45">
          Role
          <input
            name="role"
            required
            maxLength={180}
            className="border border-white/15 bg-black px-4 py-3 text-[14px] normal-case tracking-normal text-white outline-none focus:border-cyan"
          />
        </label>
      </div>
      <label className="grid gap-2 font-mono text-[11px] uppercase tracking-[0.1em] text-white/45">
        Pilot Routes
        <select
          name="routeCount"
          required
          className="border border-white/15 bg-black px-4 py-3 text-[14px] normal-case tracking-normal text-white outline-none focus:border-cyan"
        >
          {routeOptions.map((option) => (
            <option key={option}>{option}</option>
          ))}
        </select>
      </label>
      <label className="grid gap-2 font-mono text-[11px] uppercase tracking-[0.1em] text-white/45">
        Use Case
        <textarea
          name="useCase"
          required
          maxLength={1200}
          rows={5}
          className="resize-none border border-white/15 bg-black px-4 py-3 text-[14px] normal-case tracking-normal text-white outline-none focus:border-cyan"
        />
      </label>
      <button
        type="submit"
        disabled={status === "submitting"}
        className="bg-white px-7 py-3.5 font-mono text-[12px] font-bold uppercase tracking-[0.1em] text-black transition-all duration-300 hover:bg-cyan disabled:cursor-not-allowed disabled:opacity-60"
      >
        {status === "submitting" ? "Sending" : "Request Pilot"}
      </button>
      {status === "sent" ? (
        <p role="status" className="font-mono text-[11px] text-cyan">
          Pilot request received.
        </p>
      ) : null}
      {status === "error" ? (
        <p role="alert" className="font-mono text-[11px] text-red-300">
          Pilot request could not be sent. Use the pilot docs link below.
        </p>
      ) : null}
    </form>
  );
}
