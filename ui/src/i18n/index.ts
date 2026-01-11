import i18next from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";

import en from "../../locales/en.json";
import ja from "../../locales/ja.json";

export async function initI18n(): Promise<void> {
  await i18next.use(LanguageDetector).init({
    resources: {
      en: { translation: en },
      ja: { translation: ja },
    },
    fallbackLng: "en",
    debug: false,
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ["navigator", "htmlTag"],
      caches: ["localStorage"],
    },
  });
}

export function t(key: string, options?: Record<string, unknown>): string {
  return i18next.t(key, options);
}

export async function changeLanguage(lang: string): Promise<void> {
  await i18next.changeLanguage(lang);
}

export function getCurrentLanguage(): string {
  return i18next.language;
}

export default i18next;
