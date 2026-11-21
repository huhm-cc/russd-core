import type { MetadataRoute } from "next";
import { LANGS } from "@/lib/site";

export default function sitemap(): MetadataRoute.Sitemap {
  const origin = "https://russd.vercel.app";
  const pages = ["howto-use", "howto-configuration", "download-core"];
  const entries: MetadataRoute.Sitemap = [
    { url: origin, changeFrequency: "weekly", priority: 1 },
    { url: `${origin}/opensource`, changeFrequency: "monthly", priority: 0.6 },
  ];
  for (const p of pages) {
    for (const l of LANGS) {
      entries.push({
        url: `${origin}/${p}/${l.id}`,
        changeFrequency: "weekly",
        priority: 0.8,
      });
    }
  }
  return entries;
}
