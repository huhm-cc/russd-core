export type Lang = "zh" | "en" | "cn";

export type Doc = {
  title: string;
  lead: string;
  sections: {
    heading: string;
    body: string[];
    bullets?: string[];
    code?: { title: string; lang: string; text: string };
  }[];
};
