import {text as localeText} from "./i18n/Runtime";
import {loadUiPreferences,saveUiPreferences,UI_PREFERENCES_EVENT,UI_PREFERENCES_KEY} from './UiPreferences';
import {isTauriRuntime} from './tauri/runtime';
import {invoke} from "./tauri/invoke";
type Theme='light'|'dark';
export const THEME_EVENT='smart-photo-indexer:theme-changed';
let started=false,current:Theme|undefined;
let nativeAppearance=Promise.resolve();
function apply(theme:Theme){
 if(theme===current)return;current=theme;
 document.documentElement.dataset.theme=theme;document.documentElement.style.colorScheme=theme;
 if(isTauriRuntime())nativeAppearance=nativeAppearance.then(()=>invoke<void>('apply_window_appearance',{theme})).catch(()=>{window.dispatchEvent(new CustomEvent('smart-photo-indexer:appearance-notice',{detail:localeText("common", "ui.ThemeController.0eb0d1cfef71ebbb")}));});
}
/** Synchronous DOM initialization precedes React, including the editor/viewer bootstraps. */
export function startThemeController(){
 if(started)return;started=true;apply(loadUiPreferences().theme);
 const refresh=()=>apply(loadUiPreferences().theme);
 window.addEventListener(UI_PREFERENCES_EVENT,refresh);
 window.addEventListener('storage',event=>{if(event.key===null||event.key===UI_PREFERENCES_KEY)refresh();});
 window.addEventListener('pageshow',refresh);window.addEventListener('focus',refresh);
 if(isTauriRuntime())void import('@tauri-apps/api/event').then(({listen})=>listen<{theme:Theme}>(THEME_EVENT,event=>{
  const theme=event.payload.theme;if(theme!=='light'&&theme!=='dark')return;
  // Other windows may receive the event before their WebView storage catches up.
  // Persist first: preference subscribers must not immediately restore the old theme.
  try{if(loadUiPreferences().theme!==theme)saveUiPreferences({theme});else window.dispatchEvent(new Event(UI_PREFERENCES_EVENT));}
  catch{apply(theme);window.dispatchEvent(new CustomEvent('smart-photo-indexer:appearance-notice',{detail:localeText('common','ui.ThemeController.baeaa4b4ed2477f3')}));return;}
  apply(theme);
 })).catch(()=>{});
}
export async function changeTheme(theme:Theme){
 try{saveUiPreferences({theme});}catch{apply(theme);throw new Error(localeText("common", "ui.ThemeController.baeaa4b4ed2477f3"));}
 apply(theme);
 await nativeAppearance;
 if(isTauriRuntime())await import('@tauri-apps/api/event').then(({emit})=>emit(THEME_EVENT,{theme}));
}
