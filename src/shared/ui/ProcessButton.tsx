import {text as localeText, useCurrentLocale} from "../i18n/Runtime";
import {Hint} from "../help/Hint";
import {ProviderKeyDialog} from '../../modules/settings/ProviderKeyDialog';
import {useProviderReadiness} from '../../modules/settings/useProviderReadiness';
import {LocalInstallDialog} from '../../modules/localAi/LocalInstallDialog';
import { useEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { useAppActions, useAppDispatch, useAppSelector } from '../../app/state/AppStateContext';
import { Icon } from './Icon';
import { useLocalInstallation } from '../../modules/localAi/LocalModelInstallation';

export function ProcessButton({compact=false}:{compact?:boolean}){
  useCurrentLocale();
  const actions=useAppActions(),dispatch=useAppDispatch();
  const [keyOpen,setKeyOpen]=useState(false),[installOpen,setInstallOpen]=useState(false),[refresh,setRefresh]=useState(0);
  const manual=useAppSelector(s=>s.settings.processingLimits.manualQueue);
  const provider=useAppSelector(s=>s.settings.aiProviderMode);
  const cloudModel=useAppSelector(s=>(s.settings.aiProviderMode==='fal'?s.settings.fal:s.settings.openRouter).selectedModel);
  const credentialRef=useAppSelector(s=>(s.settings.aiProviderMode==='fal'?s.settings.fal:s.settings.openRouter).credentialRef);
  const cloud=useProviderReadiness(provider,cloudModel,credentialRef,refresh),needsKey=provider!=='local'&&['missing','unchecked'].includes(cloud.state),unavailable=provider!=='local'&&cloud.state==='unavailable';
  const localModel=useAppSelector(s=>s.settings.localAi.selectedModel);
  const localEndpoint=useAppSelector(s=>s.settings.localAi.endpoint);
  const local=useLocalInstallation(localModel,localEndpoint,provider==='local');
  const needsInstall=provider==='local'&&local.state!=='ready';
  const installing=needsInstall&&local.state==='installing';
  const checking=needsInstall&&['unknown','checking'].includes(local.state)||provider!=='local'&&cloud.state==='checking';
  const preparing=useAppSelector(s=>['preparing','starting'].includes(s.taskQueue.preparationState??'')||s.renamePreview.isPreparing);
  const count=useAppSelector(s=>{if(s.activeWorkspace!=='explorer'&&s.search.selectionSnapshot)return s.search.selectionSnapshot.totalCount-s.search.selectionSnapshot.excludedPaths.length;const selected=new Set(s.selectedItemIds),items=s.activeWorkspace==='explorer'?s.explorerItems:s.search.results;return items.reduce((count,item)=>count+(selected.has(item.id)&&(item.kind==='image'||item.kind==='folder')?1:0),0);});
  const anchor=useRef<HTMLDivElement>(null),menu=useRef<HTMLDivElement>(null);
  const [open,setOpen]=useState(false);const [position,setPosition]=useState({top:0,left:0});
  useEffect(()=>{
    if(!open)return;
    const outside=(e:PointerEvent)=>{if(!anchor.current?.contains(e.target as Node)&&!menu.current?.contains(e.target as Node))setOpen(false);};
    const close=()=>setOpen(false);document.addEventListener('pointerdown',outside);window.addEventListener('resize',close);
    return()=>{document.removeEventListener('pointerdown',outside);window.removeEventListener('resize',close);};
  },[open]);
  const caption=checking?localeText("common", "ui.ProcessButton.d4fc97ea7d07fe73"):unavailable?localeText("common", "ui.ProcessButton.2772dcd34d7a0caa"):needsKey?(cloud.state==='missing'?localeText("common", "ui.ProcessButton.2c712333b23194ca"):localeText("common", "ui.ProcessButton.afa837ee6e6d9fd5")):installing?localeText("common", "ui.ProcessButton.47c490ad97adb124"):needsInstall?localeText("common", "ui.ProcessButton.d97272a5124c9cbd"):preparing?localeText("common", "ui.ProcessButton.a91e67447ac99c02"):manual?localeText("common", "ui.ProcessButton.ce1ec169e94ca978"):localeText("common", "ui.ProcessButton.7efa74a5562be4d3");
  return <div ref={anchor} data-help-id="process" className={`process-button${compact?' process-button--compact':''}`}>
    <Hint message={unavailable?{id:'main.service-unavailable',args:{причина:cloud.message}}:{id:checking?'main.readiness-check':needsKey?(cloud.state==='missing'?'main.add-key':'main.check-key'):needsInstall?'main.install-local':preparing?'main.preparing':manual?'main.process-manual':'main.process-direct'}} supplement={preparing||checking||installing?{id:'common.wait',args:{действие:installing?localeText("common", "ui.ProcessButton.f7b1cba29df9b164"):checking?localeText("common", "ui.ProcessButton.b8276a36e1fcc934"):localeText("common", "ui.ProcessButton.7d7f7c7fb75ec8b3")}}:!needsInstall&&!needsKey&&count===0?{id:'main.process-empty'}:undefined} reason={checking?localeText("common", "ui.ProcessButton.0c84d3a012e5f45c"):installing?localeText("common", "ui.ProcessButton.fa0b72980bb26124"):!needsInstall&&!needsKey&&count===0?localeText("common", "ui.ProcessButton.4cb24c5c999faaf8"):undefined}><button data-tour-id={compact?undefined:'main.processing.action'} data-tour-unavailable={needsKey||needsInstall||checking||unavailable?'provider-not-ready':preparing?'busy-action':count===0?'process-selection':undefined} type="button" className="primary-action" aria-label={caption} disabled={preparing||installing||checking||unavailable||(!needsInstall&&!needsKey&&count===0)} title={needsKey||unavailable?cloud.message:needsInstall?local.message:manual?localeText("common", "ui.ProcessButton.1d50ea2ee09269e4"):localeText("common", "ui.ProcessButton.1944a529898f6671")} onClick={()=>needsKey?setKeyOpen(true):needsInstall?setInstallOpen(true):actions.queueSelectedForProcessing()}><span className="process-button__glyph" aria-hidden="true"><Icon name={needsInstall||needsKey?'plus':'play'}/></span><span className="process-button__label">{caption}</span></button></Hint>
    {!needsKey&&!needsInstall&&!checking&&!unavailable?<Hint message={{id:"main.process-mode"}}><button data-tour-id={compact?undefined:'main.processing.mode'} type="button" className="primary-action process-button__menu-trigger" aria-label={localeText("common", "ui.ProcessButton.64900f8fab18fd57")} aria-haspopup="menu" aria-expanded={open} disabled={preparing}
      onClick={()=>{const rect=anchor.current?.getBoundingClientRect();if(rect)setPosition({top:rect.bottom+4,left:Math.max(8,Math.min(rect.right-250,innerWidth-258))});setOpen(!open);}}><Icon name="down"/></button></Hint>:null}
    {open&&!needsKey&&!needsInstall&&!checking&&!unavailable?createPortal(<div ref={menu} role="menu" className="process-button__menu" style={position} onKeyDown={event=>{if(event.key==='Escape'){setOpen(false);anchor.current?.querySelector<HTMLButtonElement>('.process-button__menu-trigger')?.focus();}}}>
      {[{value:false,label:localeText("common", "ui.ProcessButton.6b4c1e57edb51c5f")},{value:true,label:localeText("common", "ui.ProcessButton.554e43a69bb938c5")}].map(mode=><Hint key={String(mode.value)} message={{id:mode.value?'main.mode-manual':'main.mode-direct'}}><button key={String(mode.value)} type="button" role="menuitemradio" aria-checked={manual===mode.value} autoFocus={manual===mode.value} onClick={()=>{dispatch({type:'settings/setManualQueue',value:mode.value});setOpen(false);anchor.current?.querySelector<HTMLButtonElement>('.process-button__menu-trigger')?.focus();}}><span className="process-button__mode-dot" aria-hidden="true">{manual===mode.value?'●':'○'}</span>{mode.label}</button></Hint>)}
    </div>,document.body):null}
    {keyOpen&&provider!=='local'?<ProviderKeyDialog key={`${provider}:${cloudModel}`} provider={provider} model={cloudModel} credentialRef={credentialRef} onClose={()=>setKeyOpen(false)} onReady={()=>setRefresh(v=>v+1)}/>:null}
    {installOpen?<LocalInstallDialog model={localModel} endpoint={localEndpoint} onClose={()=>setInstallOpen(false)}/>:null}
  </div>;
}
