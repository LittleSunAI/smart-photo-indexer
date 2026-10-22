import {i18n} from './Runtime';
import type {Locale} from './Locale';
const loaders={en:()=>import('./generated/en.json'),de:()=>import('./generated/de.json'),fr:()=>import('./generated/fr.json')};
const pending=new Map<Locale,Promise<void>>();
export function ensureLocaleResources(locale:Locale):Promise<void>{
 if(i18n.hasResourceBundle(locale,'common'))return Promise.resolve();
 const existing=pending.get(locale);if(existing)return existing;
 const load=(async()=>{if(locale==='ru')return;const module=await loaders[locale]();for(const [namespace,bundle] of Object.entries(module.default))i18n.addResourceBundle(locale,namespace,bundle);})();
 pending.set(locale,load);void load.catch(()=>pending.delete(locale));return load;
}
