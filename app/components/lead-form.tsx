"use client";

import { ChangeEvent, FormEvent, useMemo, useState } from "react";

type LeadFormData = {
  name: string;
  company: string;
  email: string;
  use_case: string;
  integration_type: string;
  timeline: string;
};

const integrationOptions = [
  "Internal API tooling",
  "Zapier / Make",
  "CRM / Sales stack",
  "Custom application",
  "Other",
];

const timelineOptions = [
  "Starting within 2 weeks",
  "Starting within 30 days",
  "Exploring this quarter",
  "Later",
];

const initialFormState: LeadFormData = {
  name: "",
  company: "",
  email: "",
  use_case: "",
  integration_type: "",
  timeline: "",
};

export function LeadForm() {
  const [formData, setFormData] = useState<LeadFormData>(initialFormState);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [statusMessage, setStatusMessage] = useState("");
  const [requestId, setRequestId] = useState("");
  const [errorMessage, setErrorMessage] = useState("");

  const isValid = useMemo(
    () =>
      Object.values(formData).every((value) => value.trim().length > 0),
    [formData]
  );

  const handleChange =
    (key: keyof LeadFormData) => (event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) => {
      setFormData((prev) => ({ ...prev, [key]: event.target.value }));
    };

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (!isValid || isSubmitting) {
      return;
    }

    setIsSubmitting(true);
    setStatusMessage("");
    setErrorMessage("");

    try {
      const response = await fetch("/api/leads", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(formData),
      });

      const payload = (await response.json().catch(() => ({}))) as {
        request_id?: string;
        error?: string;
      };

      if (!response.ok) {
        throw new Error(payload.error || "Unable to submit lead right now.");
      }

      setRequestId(payload.request_id || "submitted");
      setStatusMessage("Pilot brief received. We will reply from our onboarding desk.");
      setFormData(initialFormState);
    } catch (error: unknown) {
      setErrorMessage(
        error instanceof Error
          ? error.message
          : "Could not submit the form. Please try again."
      );
      setRequestId("");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <form
      id="pilot-lead-form"
      onSubmit={handleSubmit}
      className="flex flex-col gap-4"
      noValidate
    >
      <div className="grid gap-4 md:grid-cols-2">
        <label className="flex flex-col gap-2 font-mono text-[11px] text-white/55">
          Name
          <input
            className="w-full border border-white/15 bg-background px-4 py-3 text-[12px] text-white font-mono outline-none focus:border-cyan placeholder:text-white/20"
            type="text"
            name="name"
            value={formData.name}
            onChange={handleChange("name")}
            placeholder="Your name"
            required
          />
        </label>
        <label className="flex flex-col gap-2 font-mono text-[11px] text-white/55">
          Company
          <input
            className="w-full border border-white/15 bg-background px-4 py-3 text-[12px] text-white font-mono outline-none focus:border-cyan placeholder:text-white/20"
            type="text"
            name="company"
            value={formData.company}
            onChange={handleChange("company")}
            placeholder="Company name"
            required
          />
        </label>
      </div>

      <label className="flex flex-col gap-2 font-mono text-[11px] text-white/55">
        Work Email
        <input
          className="w-full border border-white/15 bg-background px-4 py-3 text-[12px] text-white font-mono outline-none focus:border-cyan placeholder:text-white/20"
          type="email"
          name="email"
          value={formData.email}
          onChange={handleChange("email")}
          placeholder="name@company.com"
          required
        />
      </label>

      <div className="grid gap-4 md:grid-cols-2">
        <label className="flex flex-col gap-2 font-mono text-[11px] text-white/55">
          Integration Type
          <select
            className="w-full border border-white/15 bg-background px-4 py-3 text-[12px] text-white font-mono outline-none focus:border-cyan"
            name="integration_type"
            value={formData.integration_type}
            onChange={handleChange("integration_type")}
            required
          >
            <option value="" disabled>
              Select integration type
            </option>
            {integrationOptions.map((option) => (
              <option key={option} value={option} className="bg-background">
                {option}
              </option>
            ))}
          </select>
        </label>

        <label className="flex flex-col gap-2 font-mono text-[11px] text-white/55">
          Timeline
          <select
            className="w-full border border-white/15 bg-background px-4 py-3 text-[12px] text-white font-mono outline-none focus:border-cyan"
            name="timeline"
            value={formData.timeline}
            onChange={handleChange("timeline")}
            required
          >
            <option value="" disabled>
              Select timeline
            </option>
            {timelineOptions.map((option) => (
              <option key={option} value={option} className="bg-background">
                {option}
              </option>
            ))}
          </select>
        </label>
      </div>

      <label className="flex flex-col gap-2 font-mono text-[11px] text-white/55">
        Use Case
        <textarea
          className="w-full min-h-[112px] resize-y border border-white/15 bg-background px-4 py-3 text-[12px] text-white font-mono outline-none focus:border-cyan placeholder:text-white/20"
          name="use_case"
          value={formData.use_case}
          onChange={handleChange("use_case")}
          placeholder="What integration are you evaluating?"
          rows={4}
          required
        />
      </label>

      <button
        type="submit"
        disabled={!isValid || isSubmitting}
        className="font-mono text-[11px] font-bold tracking-[0.1em] uppercase px-6 py-3.5 text-black bg-white no-underline inline-flex items-center justify-center gap-2.5 transition-all duration-300 disabled:cursor-not-allowed disabled:opacity-40 hover:bg-cyan w-fit"
      >
        {isSubmitting ? "Submitting..." : "Start Pilot Brief"}
        {!isSubmitting && (
          <svg width="14" height="14" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 17L17 7M17 7H7M17 7v10" />
          </svg>
        )}
      </button>

      {statusMessage && (
        <p className="font-mono text-[11px] text-green" role="status">
          {statusMessage} {requestId ? `(Ref: ${requestId})` : ""}
        </p>
      )}
      {errorMessage && (
        <p className="font-mono text-[11px] text-crimson" role="alert">
          {errorMessage}
        </p>
      )}
    </form>
  );
}
