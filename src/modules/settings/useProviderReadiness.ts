import{uiFeedbackText}from'../../shared/i18n/NativeMessages';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useState} from 'react';
import {forgetProviderReadiness,prepareSavedProvider} from './ProviderReadiness';
import {listen} from '@tauri-apps/api/event';
import {isTauriRuntime} from '../../shared/tauri/runtime';
import {isProviderAvailable,providerUnavailableMessage} from './ProviderAvailability';
export function useProviderReadiness(provider:string,model:string,credentialRef:string|null,refresh:number){
  useCurrentLocale();
 const [eventRevision,setEventRevision]=useState(0);
 useEffect(()=>{if(!isTauriRuntime())return;let disposed=false,off:(()=>void)|undefined;void listen('smart-photo-indexer:provider-readiness-changed',()=>{forgetProviderReadiness();setEventRevision(v=>v+1);}).then(f=>disposed?f():off=f);return()=>{disposed=true;off?.();};},[]);
 const [result,setResult]=useState<{key:string;state:'checking'|'missing'|'unchecked'|'unavailable'|'ready';message:string}>({key:'',state:'checking',message:''}),key=`${provider}|${model}|${credentialRef}`;
 useEffect(()=>{let disposed=false;if(provider==='local')return;if(!credentialRef){setResult({key,state:'missing',message:localeText("settings", "ui.useProviderReadiness.049dcfc94d2dfdbc")});return;}setResult({key,state:'checking',message:localeText("settings", "ui.useProviderReadiness.23bd9cb084ec4e60")});if(!isTauriRuntime()){setResult({key,state:'ready',message:''});return;}void prepareSavedProvider(provider,model,credentialRef).then(value=>{if(!disposed)setResult({key,state:value?.ready?'ready':'unavailable',message:value?.message??localeText("settings", "ui.useProviderReadiness.b84e7f15926dfff5")});}).catch(error=>{if(!disposed)setResult({key,state:/отклонил ключ|нет доступа к проверке|Ключ недействителен|credential|encoding/i.test(String(error))?'unchecked':'unavailable',message:String(error)});});return()=>{disposed=true;};},[key,refresh,eventRevision]);
 useEffect(()=>{if(!isTauriRuntime())return;let disposed=false,off:(()=>void)|undefined;void listen<{provider:string;credentialRef:string}>('smart-photo-indexer:provider-readiness-invalidated',e=>{if(e.payload.provider===provider&&e.payload.credentialRef===credentialRef){forgetProviderReadiness();setResult({key,state:'unchecked',message:localeText("settings", "ui.useProviderReadiness.1f225791129114a0")});}}).then(f=>disposed?f():off=f);return()=>{disposed=true;off?.();};},[key]);
 if(!isProviderAvailable(provider))return {key,state:'unavailable' as const,message:providerUnavailableMessage()};
 return result.key===key?{...result,message:uiFeedbackText('settings',result.message)}:{key,state:'checking' as const,message:localeText("settings", "ui.useProviderReadiness.d22a4d8b652db4f4")};
}
