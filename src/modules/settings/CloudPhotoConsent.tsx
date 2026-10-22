import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {cloudPhotoConsentLabel} from './CloudPhotoConsentText';
import {useCallback,useEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {Icon} from '../../shared/ui/Icon';
import catalog from '../../shared/ai/model-catalog.json';

export interface CloudPhotoConsentRequest {
  provider:'fal'|'openrouter';
  model:string;
  photoCount:number;
  selectedCount:number;
  fromSavedJob?:boolean;
}

export function useCloudPhotoConsent(){
  const [pending,setPending]=useState<(CloudPhotoConsentRequest&{id:number})|null>(null);
  const nextId=useRef(0);
  const operation=useRef<{promise:Promise<boolean>;resolve(value:boolean):void}|null>(null);
  const request=useCallback((info:CloudPhotoConsentRequest)=>{
    if(operation.current)return operation.current.promise;
    let resolve!:(value:boolean)=>void;
    const promise=new Promise<boolean>(done=>{resolve=done;});
    operation.current={promise,resolve};setPending({...info,id:++nextId.current});return promise;
  },[]);
  const settle=useCallback((value:boolean)=>{
    const current=operation.current;if(!current)return;
    operation.current=null;setPending(null);current.resolve(value);
  },[]);
  useEffect(()=>()=>{operation.current?.resolve(false);operation.current=null;},[]);
  return {pending,request,settle};
}

export function CloudPhotoConsentModal({request,onApprove,onCancel}:{request:CloudPhotoConsentRequest;onApprove():void;onCancel():void}){
  useCurrentLocale();
  const dialog=useRef<HTMLDialogElement>(null),previousFocus=useRef<HTMLElement|null>(null);
  const [accepted,setAccepted]=useState(false);
  useEffect(()=>{
    previousFocus.current=document.activeElement instanceof HTMLElement?document.activeElement:null;
    const element=dialog.current;element?.showModal();
    return()=>{element?.close();if(previousFocus.current?.isConnected)previousFocus.current.focus();};
  },[]);
  const provider=request.provider==='fal'?'Fal AI':'OpenRouter';
  const modelLabel=catalog[request.provider].find(row=>row.id===request.model)?.label??request.model;
  return createPortal(<dialog ref={dialog} className="cloud-photo-consent" aria-labelledby="cloud-consent-title" aria-describedby="cloud-consent-explanation" onCancel={event=>{event.preventDefault();onCancel();}}>
    <div className="cloud-photo-consent__heading"><span className="cloud-photo-consent__icon"><Icon name="share"/></span><h2 id="cloud-consent-title">{localeText("settings", "ui.CloudPhotoConsent.2bdb44d282314071")}</h2>
      <Hint message={{id:"common.close-dialog"}}><button type="button" className="icon-button" aria-label={localeText("settings", "ui.CloudPhotoConsent.80d25111a5c83c20")} onClick={onCancel}><Icon name="close"/></button></Hint></div>
    <p id="cloud-consent-explanation">{localeText("settings", "ui.CloudPhotoConsent.19da82d2b1185164")}{' '}<strong>{provider}</strong> {' '}{localeText("settings", "ui.CloudPhotoConsent.4e4dc13ffdaabd2f")}</p>
    <div className="cloud-photo-consent__selection"><strong>{request.photoCount>0?localeText("settings", "ui.CloudPhotoConsent.4e3ce737111da6f1", {"value1": (request.photoCount), "value2": (request.fromSavedJob?localeText("settings", "ui.CloudPhotoConsent.e87e64c6cb9aaaaa"):'')}):localeText("settings", "ui.CloudPhotoConsent.e0592b58b8992f70")}</strong><span title={request.model}>{modelLabel}</span>
      {request.photoCount===0?<small>{localeText("settings", "ui.CloudPhotoConsent.5c39decd49118bbb")}</small>:null}</div>
    <p className="cloud-photo-consent__note">{localeText("settings", "ui.CloudPhotoConsent.13007e5c682cf30e")}</p>
    <Hint message={{id:"settings.cloud-consent"}}><label className="cloud-photo-consent__agreement"><input type="checkbox" autoFocus checked={accepted} onChange={event=>setAccepted(event.target.checked)}/><span>{cloudPhotoConsentLabel()}</span></label></Hint>
    <div className="cloud-photo-consent__actions"><Hint message={{id:"common.cancel-draft"}}><button type="button" className="secondary-action" onClick={onCancel}>{localeText("settings", "ui.CloudPhotoConsent.84d7c57de897bdb2")}</button></Hint><Hint message={{id:"consent.continue"}}><button type="button" className="primary-action" disabled={!accepted} onClick={()=>{if(accepted)onApprove();}}>{localeText("settings", "ui.CloudPhotoConsent.5935b9cae4c134ea")}</button></Hint></div>
  </dialog>,document.body);
}
