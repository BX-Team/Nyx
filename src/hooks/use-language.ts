import { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';

export type Language = 'zh-CN' | 'en-US' | 'ru-RU';

export const LANGUAGES = [
  { value: 'en-US' as const, labelKey: 'languages.enUS', nativeLabelKey: 'languages.native.enUS' },
  { value: 'ru-RU' as const, labelKey: 'languages.ruRU', nativeLabelKey: 'languages.native.ruRU' },
];

export const useLanguage = () => {
  const { i18n, t } = useTranslation();

  const changeLanguage = useCallback(
    (lang: Language) => {
      i18n.changeLanguage(lang);
      localStorage.setItem('language', lang);
    },
    [i18n],
  );

  const currentLanguage = i18n.language as Language;
  const languages = useMemo(
    () =>
      LANGUAGES.map(lang => ({
        value: lang.value,
        label: t(lang.labelKey),
        nativeLabel: i18n.getFixedT(lang.value)(lang.nativeLabelKey),
      })),
    [i18n, t],
  );

  return {
    currentLanguage,
    changeLanguage,
    languages,
    t,
  };
};
