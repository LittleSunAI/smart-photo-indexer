import {nativeDisplayText,uiFeedbackText} from '../../shared/i18n/NativeMessages';
import {isProviderAvailable,providerUnavailableMessage} from './ProviderAvailability';
import {technicalError,commandErrorCode} from '../../shared/tauri/CommandError';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect, useRef, useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import { useAppDispatch } from '../../app/state/AppStateContext';
import { Icon } from '../../shared/ui/Icon';

export function ProviderCredentialFields({ provider, model, credentialRef, draft, onDraftChange }: { provider:'fal'|'openrouter'; model:string; credentialRef:string|null; draft:string; onDraftChange(value:string):void }) {
  useCurrentLocale();
  const dispatch=useAppDispatch();const label=provider==='fal'?'Fal AI':'OpenRouter';
  const key=draft,blocked=!isProviderAvailable(provider);
  const [visible,setVisible]=useState(false),[busy,setBusy]=useState(false),[message,setMessage]=useState('');
  const revision=useRef(0),operation=useRef(0),pasteRequest=useRef(0),mounted=useRef(true),context=useRef({provider,model,credentialRef});context.current={provider,model,credentialRef};
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;operation.current++;};},[]);
  useEffect(()=>{operation.current++;setBusy(false);setMessage('');},[provider,model,credentialRef]);
  const edit=(value:string)=>{revision.current++;onDraftChange(value);};
  async function save(){
    if(blocked)return;
    const submitted=key,version=revision.current,id=++operation.current;setBusy(true);setMessage('');
    try{
      const result=await invoke<{credentialRef:string;normalized?:boolean}>('save_provider_credential',{provider,key:submitted,persistent:true});
      dispatch({type:'settings/setCloudCredential',provider,credentialRef:result.credentialRef});
      if(id===operation.current&&mounted.current){if(revision.current===version){edit('');setVisible(false);}setMessage(result.normalized?localeText("settings", "ui.ProviderCredentialFields.bbcd5594b7e1259c"):localeText("settings", "ui.ProviderCredentialFields.7fee4b31b6357b0f"));}
    }catch(error){
      if(id===operation.current&&mounted.current){
        const reason=technicalError(error),code=commandErrorCode(error);
        const detail=code==='credential.empty'?localeText("settings", "ui.ProviderCredentialFields.67e22327fc6a6b3a"):code==='credential.too-long'?localeText("settings", "ui.ProviderCredentialFields.1e075a09f0f1debc"):code==='credential.invalid-characters'?localeText("settings", "ui.ProviderCredentialFields.33abe808ed196574"):reason.includes('locked')||reason.includes('busy')?localeText("settings", "ui.ProviderCredentialFields.7918ef0932b129d9"):reason.includes('protect/unlock')?localeText("settings", "ui.ProviderCredentialFields.df932ea0d6e42e65"):reason.includes('readonly')||reason.includes('read-only')?localeText("settings", "ui.ProviderCredentialFields.e4f7a23a8ad07f7c"):localeText("settings", "ui.ProviderCredentialFields.0b6b09d714c822d2");
        setMessage(localeText("settings", "ui.ProviderCredentialFields.ac86a7f12f72239b", {"value1": (detail)}));
      }
    }
    finally{if(id===operation.current&&mounted.current)setBusy(false);}
  }
  async function check(){
    if(blocked)return;
    const id=++operation.current,selected={...context.current};setBusy(true);setMessage(localeText("settings", "ui.ProviderCredentialFields.eb1df32678c15fa2"));
    try{
      const result=await invoke<{message:string}>('get_ai_provider_status',{provider,model:selected.model,credentialRef:selected.credentialRef});
      if(mounted.current&&id===operation.current&&selected.model===context.current.model&&selected.credentialRef===context.current.credentialRef)setMessage(result.message);
    }catch(error){if(mounted.current&&id===operation.current&&selected.model===context.current.model&&selected.credentialRef===context.current.credentialRef)setMessage(String(error));}finally{if(mounted.current&&id===operation.current)setBusy(false);}
  }
  async function paste(){
    if(blocked)return;
    const version=revision.current,id=++pasteRequest.current,selectedProvider=provider;
    const current=()=>mounted.current&&id===pasteRequest.current&&version===revision.current&&selectedProvider===context.current.provider;
    try{const text=await invoke<string>('read_clipboard_text');if(current()){edit(text.trim());setVisible(false);setMessage(localeText("settings", "ui.ProviderCredentialFields.86598c3c91ede5c7"));}}
    catch{if(current())setMessage(localeText("settings", "ui.ProviderCredentialFields.cbf3990a8ca142e2"));}
  }
  return <section className="credential-fields" data-provider-disabled={blocked||undefined} data-testid="provider-credential-fields" aria-label={localeText("settings", "ui.ProviderCredentialFields.f4ffa534db674435", {"value1": (label)})}>
    {blocked?<p className="settings-muted" role="status">{providerUnavailableMessage()}</p>:null}
    <div className="credential-fields__heading"><strong>{label}</strong><span>{credentialRef?localeText("settings", "ui.ProviderCredentialFields.baa21885fb378ef9"):localeText("settings", "ui.ProviderCredentialFields.4b7037593a5b04f1")}</span></div>
    <div className="credential-fields__input">
      <Hint message={{id:"key.input"}}><input disabled={blocked} aria-label={localeText("settings", "ui.ProviderCredentialFields.e19145cd47fa2c81", {"value1": (label)})} type={visible?'text':'password'} value={key} onChange={event=>edit(event.target.value)} autoComplete="off" spellCheck={false} placeholder={localeText("settings", "ui.ProviderCredentialFields.4b71dc68b2fb895d")} maxLength={4096}/></Hint>
      <Hint message={{id:"key.paste"}}><button type="button" disabled={blocked} aria-label={localeText("settings", "ui.ProviderCredentialFields.7c70831f86c19651", {"value1": (label)})} title={localeText("settings", "ui.ProviderCredentialFields.4d00c2b4a9521c73")} onClick={()=>void paste()}><Icon name="clipboard"/></button></Hint>
      <Hint message={{id:visible?'key.hide':'key.show'}}><button type="button" disabled={blocked} aria-label={visible?localeText("settings", "ui.ProviderCredentialFields.06dea58bcd8f1168", {"value1": (label)}):localeText("settings", "ui.ProviderCredentialFields.90f6b6402e480411", {"value1": (label)})} title={visible?localeText("settings", "ui.ProviderCredentialFields.79ac762713732241"):localeText("settings", "ui.ProviderCredentialFields.a7a079a4bac0cf35")} aria-pressed={visible} onClick={()=>setVisible(!visible)}><Icon name={visible?'eyeOff':'eye'}/></button></Hint>
    </div>
    <div className="settings-action-row"><Hint message={{id:"key.save"}} supplement={busy?{id:"key.wait"}:undefined}><button type="button" className="primary-action" disabled={blocked||busy||!key.trim()} onClick={()=>void save()}>{localeText("settings", "ui.ProviderCredentialFields.54d622cdd0057b1b")}{' '}{label}</button></Hint>
      <Hint message={{id:"key.check-service"}}><button type="button" disabled={blocked||busy||!credentialRef} onClick={()=>void check()}>{localeText("settings", "ui.ProviderCredentialFields.577d23602b12d5a8")}{' '}{label}</button></Hint></div>
    {message?<p className="settings-check" role="status">{uiFeedbackText('settings',message)}</p>:null}
  </section>;
}
