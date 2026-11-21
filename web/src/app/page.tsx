import Link from "next/link";
import { SiteFooter, SiteHeader } from "@/components/site-chrome";
import { buttonVariants } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { LANGS } from "@/lib/site";
import { cn } from "@/lib/utils";

export default function Home() {
  return (
    <div className="flex min-h-full flex-col">
      <SiteHeader lang="zh" />
      <main className="mx-auto w-full max-w-5xl flex-1 px-4 py-12">
        <p className="text-sm tracking-[0.2em] text-muted-foreground uppercase">
          Open source language
        </p>
        <h1 className="mt-3 max-w-3xl text-5xl font-semibold tracking-tight text-balance">
          Russd
        </h1>
        <p className="mt-4 max-w-2xl text-lg text-muted-foreground">
          開源、語法接近 Rust、預設更安全。說明站部署在 Vercel。選語言開始：如何使用、如何設定、下載核心。
        </p>
        <div className="mt-8 flex flex-wrap gap-3">
          <Link href="/howto-use/zh" className={cn(buttonVariants())}>
            如何使用 · 繁中
          </Link>
          <Link
            href="/download-core/en"
            className={cn(buttonVariants({ variant: "outline" }))}
          >
            Download core
          </Link>
          <Link
            href="/opensource"
            className={cn(buttonVariants({ variant: "ghost" }))}
          >
            Open source
          </Link>
        </div>
        <div className="mt-12 grid gap-4 sm:grid-cols-3">
          {[
            { href: "/howto-use", title: "How to use", zh: "如何使用", cn: "如何使用" },
            {
              href: "/howto-configuration",
              title: "Configuration",
              zh: "如何設定",
              cn: "如何配置",
            },
            { href: "/download-core", title: "Download", zh: "下載核心", cn: "下载核心" },
          ].map((card) => (
            <Card key={card.href}>
              <CardHeader>
                <CardTitle>{card.title}</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-2 text-sm">
                {LANGS.map((l) => (
                  <Link
                    key={l.id}
                    href={`${card.href}/${l.id}`}
                    className="text-foreground/80 underline-offset-4 hover:underline"
                  >
                    {l.label}
                  </Link>
                ))}
              </CardContent>
            </Card>
          ))}
        </div>
      </main>
      <SiteFooter lang="zh" />
    </div>
  );
}
