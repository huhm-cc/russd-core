import { CopyBlock } from "@/components/copy-block";
import { LangPills, SiteFooter, SiteHeader } from "@/components/site-chrome";
import { ui, type Lang } from "@/lib/site";

export function DocPage({
  lang,
  base,
  title,
  lead,
  sections,
}: {
  lang: Lang;
  base: string;
  title: string;
  lead: string;
  sections: {
    heading: string;
    body: string[];
    code?: { title: string; lang: string; text: string };
  }[];
}) {
  const t = ui[lang];
  return (
    <div className="flex min-h-full flex-col">
      <SiteHeader lang={lang} />
      <main className="mx-auto w-full max-w-5xl flex-1 px-4 py-10">
        <LangPills base={base} lang={lang} />
        <h1 className="mt-6 text-4xl font-semibold tracking-tight text-balance">{title}</h1>
        <p className="mt-3 max-w-2xl text-lg text-muted-foreground">{lead}</p>
        <div className="mt-10 space-y-10">
          {sections.map((s) => (
            <section key={s.heading} className="space-y-3">
              <h2 className="text-xl font-semibold tracking-tight">{s.heading}</h2>
              {s.body.map((p) => (
                <p key={p} className="max-w-2xl leading-7 text-foreground/90">
                  {p}
                </p>
              ))}
              {s.code ? (
                <CopyBlock
                  title={s.code.title}
                  text={s.code.text}
                  copyLabel={t.copy}
                  copiedLabel={t.copied}
                />
              ) : null}
            </section>
          ))}
        </div>
      </main>
      <SiteFooter lang={lang} />
    </div>
  );
}
