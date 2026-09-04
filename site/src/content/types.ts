export type Language = "fr" | "en";

export type ExternalLink = {
  href: string;
  label: string;
};

export type SiteCopy = {
  lang: Language;
  alternateLang: Language;
  alternateLabel: string;
  title: string;
  description: string;
  nav: { features: string; install: string; technology: string; faq: string; github: string };
  hero: {
    eyebrow: string;
    title: string;
    accent: string;
    body: string;
    download: string;
    availability: string;
    install: string;
    screenshotAsset: string;
    screenshotAlt: string;
    screenshotCaption: string;
  };
  updates: {
    eyebrow: string; title: string; body: string;
    items: Array<{ title: string; body: string }>;
    link: ExternalLink;
  };
  promise: {
    eyebrow: string;
    title: string;
    body: string;
    items: Array<{ number: string; title: string; body: string }>;
  };
  companions: {
    eyebrow: string;
    title: string;
    body: string;
    points: string[];
    screenshotAsset: string;
    screenshotAlt: string;
  };
  install: {
    eyebrow: string;
    title: string;
    intro: string;
    steps: Array<{ title: string; body: string; links?: ExternalLink[] }>;
    requirementsTitle: string;
    requirements: Array<{ label: string; value: string; links?: ExternalLink[] }>;
    capacityTitle: string;
    capacities: Array<{ memory: string; bots: string }>;
    note: string;
  };
  technology: {
    eyebrow: string;
    title: string;
    body: string;
    nodes: Array<{ label: string; detail: string }>;
    facts: Array<{ title: string; body: string }>;
  };
  local: {
    eyebrow: string;
    title: string;
    body: string;
    points: string[];
  };
  faq: {
    eyebrow: string;
    title: string;
    items: Array<{ question: string; answer: string }>;
  };
  footer: { independent: string; license: string; documentation: string; source: string; portfolio: string };
};
