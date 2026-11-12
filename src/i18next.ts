import i18n from "i18next";
import { initReactI18next } from "react-i18next";

const languages = [
  ["az", "Azərbaycan"],
  ["en", "English"],
  ["el", "Ελληνικά"],
  ["am", "Հայերեն"],
  ["es", "Español"],
  ["it", "Italiano"],
  ["de", "Deutsch"],
  ["de_ch", "Schweizerdeutsch"],
  ["fr", "Français"],
  ["pl", "Polski"],
  ["nl", "Nederlands"],
  ["vi", "Tiếng Việt"],
  ["ru", "Русский"],
  ["uk", "Українська"],
  ["ro", "Română"],
  ["ar", "العربية"],
  ["tr", "Türkçe"],
  ["zh_tw", "Traditional Chinese （繁體中文)"],
  ["zh_cn", "Simplified Chinese （简体中文)"],
  ["ko", "한국어"],
  ["zh_hk", "Cantonese （粵語)"],
  ["ja", "日本語"],
  ["cs_cz", "Čeština"],
  ["sv", "Svenska"],
  ["hu", "Magyar"],
  ["kh", "ភាសាខ្មែរ"],
  ["id", "Bahasa Indonesia"],
  ["pt_br", "Português (Brasileiro)"]
] as const;

export const sortedLanguages = [...languages].sort((a, b) =>
  a[0].localeCompare(b[0]),
);

type TranslationResource = Record<string, unknown>;

const localeModules = import.meta.glob<{ default: TranslationResource }>(
  "./locales/*.json",
  {
    eager: true,
  },
);

const resources = Object.fromEntries(
  Object.entries(localeModules).flatMap(([path, module]) => {
    const lang = path.match(/\/([\w-]+)\.json$/)?.[1];
    if (!lang) return [];

    return [[lang, { translation: module.default }]];
  }),
);

// Locale codes differ between the OS ("zh-CN", "pt-BR") and the bundled files
// ("zh_cn", "pt_br"), so map the system language onto a resource key before
// handing it to i18next. Without this the detector's value never matched and
// everything fell back to English.
const normalize = (raw?: string | null) => {
  if (!raw) return undefined;
  const lower = raw.toLowerCase().replace(/-/g, "_");
  if (lower in resources) return lower;
  const base = lower.split("_")[0];
  if (base in resources) return base;
  const alias: Record<string, string> = {
    zh: "zh_cn",
    pt: "pt_br",
    cs: "cs_cz",
  };
  return alias[base] in resources ? alias[base] : undefined;
};

const systemLanguage = () => {
  const candidates = [...(navigator.languages ?? []), navigator.language];
  for (const candidate of candidates) {
    const matched = normalize(candidate);
    if (matched) return matched;
  }
  return undefined;
};

const initialLanguage =
  normalize(localStorage.getItem("i18nextLng")) ?? systemLanguage() ?? "en";

i18n.use(initReactI18next).init({
  fallbackLng: "en",
  lng: initialLanguage,
  interpolation: {
    escapeValue: false,
  },
  resources,
});

export default i18n;
