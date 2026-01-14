/**
 * i18n configuration
 */

export const i18nConfig = {
  fallbackLng: 'en',
  supportedLngs: ['ja', 'en'],
  defaultNS: 'translation',
  interpolation: {
    escapeValue: false, // React already escapes by default
  },
  detection: {
    order: ['localStorage', 'navigator'],
    caches: ['localStorage'],
    lookupLocalStorage: 'i18nextLng',
  },
};
