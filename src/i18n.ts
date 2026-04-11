import dayjs from 'dayjs';
import localizedFormat from 'dayjs/plugin/localizedFormat';
import relativeTime from 'dayjs/plugin/relativeTime';
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import 'dayjs/locale/ru';
import 'dayjs/locale/en';

import enUS from './locales/en-US';
import ruRU from './locales/ru-RU';

const resources = {
  'en-US': { translation: enUS },
  'ru-RU': { translation: ruRU },
};

const getSavedLanguage = (): string => {
  const saved = localStorage.getItem('language');
  if (saved && ['en-US', 'ru-RU'].includes(saved)) {
    return saved;
  }

  const systemLang = navigator.language || 'en-US';
  if (systemLang.startsWith('ru')) return 'ru-RU';
  if (systemLang.startsWith('en')) return 'en-US';

  return 'en-US';
};

const savedLanguage = getSavedLanguage();

dayjs.extend(relativeTime);
dayjs.extend(localizedFormat);
const dayjsLocaleMap: Record<string, string> = {
  'en-US': 'en',
  'ru-RU': 'ru',
};
dayjs.locale(dayjsLocaleMap[savedLanguage] || 'en');

i18n.use(initReactI18next).init({
  resources,
  lng: savedLanguage,
  fallbackLng: 'en-US',
  interpolation: {
    escapeValue: false,
  },
  react: {
    useSuspense: false,
  },
});

export default i18n;
