"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";

export function CopyBlock({
  title,
  text,
  copyLabel,
  copiedLabel,
}: {
  title: string;
  text: string;
  copyLabel: string;
  copiedLabel: string;
}) {
  const [copied, setCopied] = useState(false);

  async function onCopy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    } catch {
      setCopied(false);
    }
  }

  return (
    <div className="overflow-hidden rounded-lg border border-border bg-card">
      <div className="flex items-center justify-between gap-3 border-b border-border px-3 py-2">
        <p className="text-xs font-medium tracking-wide text-muted-foreground uppercase">
          {title}
        </p>
        <Button type="button" size="sm" variant="outline" onClick={onCopy}>
          {copied ? copiedLabel : copyLabel}
        </Button>
      </div>
      <pre className="overflow-x-auto p-4 text-sm leading-relaxed font-mono text-foreground">
        <code>{text}</code>
      </pre>
    </div>
  );
}
