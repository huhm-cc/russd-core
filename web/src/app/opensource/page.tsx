import Link from "next/link";
import { SiteFooter, SiteHeader } from "@/components/site-chrome";
import { GITHUB_CORE, opensource } from "@/lib/site";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export const metadata = {
  title: "Open source · Russd",
};

export default function OpenSourcePage() {
  return (
    <div className="flex min-h-full flex-col">
      <SiteHeader lang="en" />
      <main className="mx-auto w-full max-w-5xl flex-1 px-4 py-10">
        <Badge variant="secondary">{opensource.license}</Badge>
        <h1 className="mt-4 text-4xl font-semibold tracking-tight">{opensource.title}</h1>
        <p className="mt-3 max-w-2xl text-muted-foreground">
          Source:{" "}
          <a className="underline underline-offset-4" href={GITHUB_CORE}>
            {GITHUB_CORE}
          </a>
        </p>
        <div className="mt-8 grid gap-4">
          {opensource.points.map((p) => (
            <Card key={p.en}>
              <CardHeader>
                <CardTitle className="text-base font-medium leading-7">{p.zh}</CardTitle>
              </CardHeader>
              <CardContent className="space-y-2 text-sm text-muted-foreground">
                <p>{p.en}</p>
                <p>{p.cn}</p>
              </CardContent>
            </Card>
          ))}
        </div>
        <p className="mt-8">
          <Link href="/howto-use/en" className="underline underline-offset-4">
            How to use
          </Link>
          {" · "}
          <Link href="/download-core/en" className="underline underline-offset-4">
            Download
          </Link>
        </p>
      </main>
      <SiteFooter lang="en" />
    </div>
  );
}
