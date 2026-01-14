/**
 * i18n initialization
 */

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';

import { i18nConfig } from './config';
import ja from '../../locales/ja.json';
import en from '../../locales/en.json';

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    ...i18nConfig,
    resources: {
      ja: { translation: ja },
      en: { translation: en },
    },
  });

export default i18n;
