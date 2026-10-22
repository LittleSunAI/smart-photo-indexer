import {useSyncExternalStore} from 'react';
import {i18n} from './Runtime';
import {normalizeLocale,type Locale} from './Locale';
import {ensureLocaleResources} from './LocaleResources';
import {readUiLocale,saveUiPreferences,UI_PREFERENCES_EVENT,UI_PREFERENCES_KEY} from '../UiPreferences';
import {isTauriRuntime} from '../tauri/runtime';

export interface LocaleState{locale:Locale;pending:Locale|null;error:'load'|'save'|'read'|null}
interface LocalePorts{read():Locale;save(locale:Locale):void;load(locale:Locale):Promise<void>;apply(locale:Locale):void;current():Locale;broadcast():void}
/** One owner per window. A later request/read invalidates every older load. */
export function createLocaleController(ports:LocalePorts){
 let state:LocaleState={locale:ports.current(),pending:null,error:null},generation=0,stored:Locale|undefined,active:Promise<boolean>|null=null;
 const listeners=new Set<()=>void>();
 const update=(change:Partial<LocaleState>)=>{state={...state,...change};for(const listener of listeners)listener();};
 function perform(locale:Locale,persist:boolean):Promise<boolean>{
  const ticket=++generation;update({pending:locale,error:null});
  const work=(async()=>{
   try{await ports.load(locale);}catch{if(ticket===generation){stored=undefined;update({pending:null,error:'load'});}return false;}
   if(ticket!==generation)return false;
   if(persist){const previous=stored;stored=locale;try{ports.save(locale);}catch{stored=previous;update({pending:null,error:'save'});return false;}}
   let fresh:Locale;try{fresh=ports.read();}catch{update({pending:null,error:'read'});return false;}
   if(ticket!==generation)return false;
   if(fresh!==locale){stored=undefined;return synchronize(true);}
   stored=fresh;ports.apply(fresh);update({locale:fresh,pending:null,error:null});if(persist)ports.broadcast();return true;
  })();
  active=work;void work.finally(()=>{if(active===work)active=null;});return work;
 }
 function synchronize(force=false):Promise<boolean>{
  let locale:Locale;try{locale=ports.read();}catch{update({error:'read'});return Promise.resolve(false);}
  if(!force&&locale===stored&&(state.pending===locale||state.locale===locale&&!state.error))return Promise.resolve(true);
  stored=locale;return perform(locale,false);
 }
 return {getSnapshot:()=>state,subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener);};},request:(value:unknown)=>perform(normalizeLocale(value),true),synchronize,async idle(){while(active)await active;}};
}
export const LOCALE_EVENT='smart-photo-indexer:locale-changed';
export const localeController=createLocaleController({
 read:readUiLocale,save:locale=>{saveUiPreferences({locale});},load:ensureLocaleResources,current:()=>normalizeLocale(i18n.resolvedLanguage),
 apply:locale=>{void i18n.changeLanguage(locale);document.documentElement.lang=locale;},
 broadcast:()=>{if(isTauriRuntime())void import('@tauri-apps/api/event').then(({emit})=>emit(LOCALE_EVENT)).catch(()=>{});},
});
let started=false;
export async function startLocaleController(){
 if(!started){started=true;
  const refresh=()=>{void localeController.synchronize();};
  window.addEventListener(UI_PREFERENCES_EVENT,refresh);
  window.addEventListener('storage',event=>{if(event.key===null||event.key===UI_PREFERENCES_KEY)refresh();});
  window.addEventListener('focus',refresh);window.addEventListener('pageshow',refresh);
  if(isTauriRuntime())void import('@tauri-apps/api/event').then(({listen})=>listen(LOCALE_EVENT,refresh)).catch(()=>{});
  await localeController.synchronize(true);
 }
 await localeController.idle();return localeController.getSnapshot();
}
export function useLocaleState(){return useSyncExternalStore(localeController.subscribe,localeController.getSnapshot,localeController.getSnapshot);}
