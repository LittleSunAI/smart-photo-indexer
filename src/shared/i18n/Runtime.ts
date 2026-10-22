import {createInstance} from 'i18next';
import {initReactI18next,useTranslation} from 'react-i18next';
import russian from './generated/ru.json';
import type {ResourceKeys} from './generated/ResourceKeys';

export const i18n=createInstance();
void i18n.use(initReactI18next).init({
 lng:'ru',fallbackLng:'ru',supportedLngs:['ru','en','de','fr'],
 resources:{ru:russian},ns:Object.keys(russian),defaultNS:'common',keySeparator:false,
 initAsync:false,interpolation:{escapeValue:false},react:{useSuspense:false},
});
export function text<N extends keyof ResourceKeys>(ns:N,key:ResourceKeys[N],args?:Record<string,unknown>){
 return i18n.t(String(key),{...Object.fromEntries(Object.entries(args??{}).map(([k,v])=>[k,v==null?String(v):v])),ns}) as string;
}
/** Subscribe once per consumer component; labels are resolved at render time. */
export function useLocaleText(){useTranslation('common',{i18n});return text;}
export function useCurrentLocale(){useTranslation('common',{i18n});return i18n.resolvedLanguage??'ru';}

export type RichToken=string|{kind:'arg';name:string}|{kind:'strong'|'code'|'kbd';children:RichToken[]};
export type RichEntry=RichToken[][];
function tokensValid(value:unknown):value is RichToken[]{
 return Array.isArray(value)&&value.every(token=>typeof token==='string'||token&&typeof token==='object'&&(
  token.kind==='arg'?typeof token.name==='string': ['strong','code','kbd'].includes(token.kind)&&tokensValid(token.children)));
}
function richValid(value:unknown):value is RichEntry{return Array.isArray(value)&&value.length>0&&value.every(tokensValid);}
/** Whole-entry fallback: never splice paragraphs from two different languages. */
export function richMessage(id:string):RichEntry{
 const locale=i18n.resolvedLanguage??'ru';
 const entry=i18n.getResource(locale,'help',id);
 if(richValid(entry))return entry;
 const fallback=i18n.getResource('ru','help',id);
 if(richValid(fallback))return fallback;
 throw Error(`Missing or invalid rich message: ${id}`);
}

