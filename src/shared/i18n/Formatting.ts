import {i18n} from './Runtime';
import type {ResourceKeys} from './generated/ResourceKeys';
export function localeTag(){return ({ru:'ru-RU',en:'en-US',de:'de-DE',fr:'fr-FR'} as Record<string,string>)[i18n.resolvedLanguage??'ru']??'ru-RU';}
type Family<T> = T extends `${infer K}_${'one'|'few'|'many'|'other'}`?K:never;
export type PluralKey=Family<ResourceKeys['plurals']>;
export function plural(key:PluralKey,count:number,args:Record<string,string|number>={}){return i18n.t(key,{ns:'plurals',...args,count}) as string;}
export function displayNumber(value:number,options:Intl.NumberFormatOptions={}){return new Intl.NumberFormat(localeTag(),options).format(value);}
