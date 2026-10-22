import {invoke} from "../../shared/tauri/invoke";
import {listen} from '@tauri-apps/api/event';
import {useEffect,useSyncExternalStore} from 'react';
import {isTauriRuntime} from '../../shared/tauri/runtime';
export interface ProfilePreferences{version:number;revision:number;cloudConsent:boolean;recursiveProcessing:boolean;analysisCopy:{enabled:boolean;destination:'beside'|'folder';folder:string}}
type Patch=Partial<Pick<ProfilePreferences,'cloudConsent'|'recursiveProcessing'|'analysisCopy'>>;
let current:ProfilePreferences={version:1,revision:0,cloudConsent:false,recursiveProcessing:false,analysisCopy:{enabled:false,destination:'beside',folder:''}},listener:Promise<unknown>|null=null;
const subscribers=new Set<()=>void>();
function publish(value:ProfilePreferences){if(value.revision<current.revision)return;current=value;subscribers.forEach(f=>f());}
function observe(){if(!listener&&isTauriRuntime())listener=listen<ProfilePreferences>('smart-photo-indexer:profile-preferences-changed',event=>publish(event.payload)).catch(error=>{listener=null;throw error;});return listener;}
export async function readProfilePreferences(){await observe();if(isTauriRuntime())publish(await invoke<ProfilePreferences>('get_profile_preferences'));return current;}
export async function initializeProfilePreferences(legacyConsent:boolean){await observe();if(isTauriRuntime())publish(await invoke<ProfilePreferences>('initialize_profile_preferences',{legacyConsent}));return current;}
export async function patchProfilePreferences(patch:Patch){const base=await readProfilePreferences();if(!isTauriRuntime()){publish({...base,...patch,revision:base.revision+1});return current;}try{const value=await invoke<ProfilePreferences>('patch_profile_preferences',{expectedRevision:base.revision,patch});publish(value);return value;}catch(error){await readProfilePreferences();throw error;}}
export const subscribeProfilePreferences=(f:()=>void)=>{subscribers.add(f);return()=>{subscribers.delete(f);};};
export const profilePreferencesSnapshot=()=>current;
export function useProfilePreferences(){const value=useSyncExternalStore(subscribeProfilePreferences,profilePreferencesSnapshot);useEffect(()=>{void readProfilePreferences().catch(()=>{});},[]);return value;}
