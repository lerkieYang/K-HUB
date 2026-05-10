import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import zh from './zh';
import en from './en';

export type Locale = 'zh' | 'en';

const translations: Record<Locale, Record<string, string>> = { zh, en };

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}

export const useI18n = create<I18nState>()(
  persist(
    (set, get) => ({
      locale: 'zh',
      setLocale: (locale) => set({ locale }),
      t: (key: string, params?: Record<string, string | number>) => {
        const { locale } = get();
        let text = translations[locale]?.[key] || translations['zh']?.[key] || key;
        if (params) {
          Object.entries(params).forEach(([k, v]) => {
            text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
          });
        }
        return text;
      },
    }),
    {
      name: 'khub-i18n',
    }
  )
);
