import {useEffect,useState} from 'react';
import {loadUiPreferences,UI_PREFERENCES_KEY,UI_PREFERENCES_EVENT} from './UiPreferences';
export function useUiPreferences(){
 const [preferences,setPreferences]=useState(loadUiPreferences);
 useEffect(()=>{const refresh=()=>setPreferences(loadUiPreferences()),storage=(e:StorageEvent)=>{if(e.key===UI_PREFERENCES_KEY||e.key===null)refresh();};window.addEventListener(UI_PREFERENCES_EVENT,refresh);window.addEventListener('storage',storage);refresh();return()=>{window.removeEventListener(UI_PREFERENCES_EVENT,refresh);window.removeEventListener('storage',storage);};},[]);
 return preferences;
}
