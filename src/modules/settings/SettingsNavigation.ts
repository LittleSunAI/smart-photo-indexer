import{useSyncExternalStore}from'react';
import type{SettingsTab}from'../../shared/SettingsTabs';
export{settingsTabs,type SettingsTab}from'../../shared/SettingsTabs';
const listeners=new Set<()=>void>();let state:{serial:number;tab:SettingsTab|null}={serial:0,tab:null};
/** A one-shot explicit entry. Ordinary opening does not replace the last tab. */
export function requestSettingsTab(tab:SettingsTab){state={serial:state.serial+1,tab};listeners.forEach(f=>f());}
export function useSettingsEntry(){return useSyncExternalStore(f=>{listeners.add(f);return()=>{listeners.delete(f);};},()=>state);}
