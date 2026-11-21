import Link from "next/link";
import { LANGS, NAV, ui, type Lang } from "@/lib/site";

export function SiteHeader({ lang }: { lang: Lang }) {
  const t = ui[lang];
  return (
    <header className="border-b border-border bg-background/90 backdrop-blur">
      <div className="mx-auto flex max-w-5xl flex-col gap-3 px-4 py-4 sm:flex-row sm:items-center sm:justify-between">
        <Link href="/" className="font-semibold tracking-tight">
          {t.brand}
          <span className="ml-2 text-sm font-normal text-muted-foreground">
            {t.tagline}
          </span>
        </Link>
        <nav className="flex flex-wrap gap-x-4 gap-y-2 text-sm">
          {NAV.map((item) => (
            <Link
              key={item.href}
              href={item.href === "/opensource" ? "/opensource" : `${item.href}/${lang}`}
              className="text-foreground/80 hover:text-foreground"
            >
              {t[item.key]}
            </Link>
          ))}
        </nav>
      </div>
    </header>
  );
}

export function LangPills({
  base,
  lang,
}: {
  base: string;
  lang: Lang;
}) {
  return (
    <p className="flex flex-wrap gap-2 text-sm">
      {LANGS.map((l) => (
        <Link
          key={l.id}
          href={`${base}/${l.id}`}
          className={
            l.id === lang
              ? "rounded-full bg-foreground px-3 py-1 text-background"
              : "rounded-full border border-border px-3 py-1 text-foreground/80"
          }
        >
          {l.label}
        </Link>
      ))}
    </p>
  );
}

export function SiteFooter({ lang }: { lang: Lang }) {
  const t = ui[lang];
  return (
    <footer className="mt-auto border-t border-border">
      <div className="mx-auto max-w-5xl px-4 py-8 text-sm text-muted-foreground">
        <p>{t.deployNote}</p>
        <p className="mt-2">GPL-3.0 · Russd core</p>
      </div>
    </footer>
  );
}
