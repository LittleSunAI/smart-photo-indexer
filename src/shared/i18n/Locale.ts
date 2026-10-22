export const supportedLocales=['ru','en','de','fr'] as const;
export type Locale=typeof supportedLocales[number];
export const localeNames:Record<Locale,string>={ru:'Русский',en:'English',de:'Deutsch',fr:'Français'};
export const availableLocales:readonly Locale[]=supportedLocales;
export function normalizeLocale(value:unknown):Locale{return supportedLocales.includes(value as Locale)?value as Locale:'ru';}
