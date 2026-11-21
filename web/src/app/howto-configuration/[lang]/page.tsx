import { notFound } from "next/navigation";
import { DocPage } from "@/components/doc-page";
import { howtoConfig, isLang } from "@/lib/site";

export function generateStaticParams() {
  return [{ lang: "zh" }, { lang: "en" }, { lang: "cn" }];
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ lang: string }>;
}) {
  const { lang } = await params;
  if (!isLang(lang)) return {};
  return { title: howtoConfig[lang].title };
}

export default async function Page({
  params,
}: {
  params: Promise<{ lang: string }>;
}) {
  const { lang } = await params;
  if (!isLang(lang)) notFound();
  const doc = howtoConfig[lang];
  return (
    <DocPage
      lang={lang}
      base="/howto-configuration"
      title={doc.title}
      lead={doc.lead}
      sections={doc.sections}
    />
  );
}
