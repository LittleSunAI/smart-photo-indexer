import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useState} from 'react';
import {useAppActions,useAppSelector} from '../../app/state/AppStateContext';
import {Icon} from '../../shared/ui/Icon';

export function CollectionsWorkspace({previewCollectionId}:{previewCollectionId?:string}={}){
  useCurrentLocale();
  const actions=useAppActions(),collections=useAppSelector(state=>state.collections),actualCurrent=useAppSelector(state=>state.search.collectionId);
  const current=previewCollectionId??actualCurrent;
  const [creating,setCreating]=useState(false),[name,setName]=useState(''),[busy,setBusy]=useState(false);
  const selected=collections.items.find(item=>item.id===current);
  const create=async()=>{if(!name.trim()||busy)return;setBusy(true);try{const result=await actions.createCollection(name.trim());if(result?.collection){setName('');setCreating(false);}}finally{setBusy(false);}};
  return <section className={`collections-workspace${selected?' collections-workspace--selected':''}`} aria-label={localeText("main", "ui.CollectionsWorkspace.b79a9ca06e621299")}>
    <div className="collections-workspace__heading">
      {selected?<><Hint message={{id:"collection.all"}}><button type="button" className="secondary-action" onClick={()=>actions.openCollection('')}><Icon name="left"/>{localeText("main", "ui.CollectionsWorkspace.941b807938afbec3")}</button></Hint><Hint message={{id:"collection.choose"}}><label><span className="sr-only">{localeText("main", "ui.CollectionsWorkspace.6c08483e4093066f")}</span><select aria-label={localeText("main", "ui.CollectionsWorkspace.3df25e5d105c6684")} value={selected.id} onChange={event=>actions.openCollection(event.target.value)}>{collections.items.map(item=><option key={item.id} value={item.id}>{item.name} · {item.imageCount}</option>)}</select></label></Hint></>:<strong>{localeText("main", "ui.CollectionsWorkspace.8e5098a9c290a543")}</strong>}
      <Hint message={{id:"collection.refresh"}}><button type="button" className="icon-button" aria-label={localeText("main", "ui.CollectionsWorkspace.29f99a3b2969ea44")} title={localeText("main", "ui.CollectionsWorkspace.2645a6070135b189")} disabled={collections.isLoading} onClick={()=>void actions.loadCollections()}><Icon name="refresh"/></button></Hint>
      <Hint message={{id:"collection.create"}}><button type="button" className="secondary-action" onClick={()=>setCreating(!creating)}><Icon name="plus"/>{localeText("main", "ui.CollectionsWorkspace.49a58144570f59d6")}</button></Hint>
    </div>
    {creating?<form className="collections-workspace__create" onSubmit={event=>{event.preventDefault();void create();}}><Hint message={{id:"collection.name"}}><input aria-label={localeText("main", "ui.CollectionsWorkspace.faa637392f520337")} placeholder={localeText("main", "ui.CollectionsWorkspace.61f7161a89383506")} autoFocus maxLength={160} value={name} onChange={event=>setName(event.target.value)} disabled={busy}/></Hint><Hint message={{id:"collection.create"}}><button type="submit" className="primary-action" disabled={busy||!name.trim()}>{localeText("main", "ui.CollectionsWorkspace.49a58144570f59d6")}</button></Hint><Hint message={{id:"common.cancel-draft"}}><button type="button" className="icon-button" aria-label={localeText("main", "ui.CollectionsWorkspace.29b7da13a8eaa8bc")} onClick={()=>setCreating(false)}><Icon name="close"/></button></Hint></form>:null}
    {!selected?<><p>{localeText("main", "ui.CollectionsWorkspace.4358e26301819b1e")}</p><div className="collections-workspace__list">{collections.items.map(item=><Hint key={item.id} message={{id:"collection.open",args:{название:item.name}}}><button type="button" key={item.id} onClick={()=>actions.openCollection(item.id)}><Icon name="collection"/><span><strong>{item.name}</strong><small>{item.imageCount} {' '}{localeText("main", "ui.CollectionsWorkspace.1b4719b020f01d86")}</small></span><Icon name="right"/></button></Hint>)}</div>{!collections.items.length&&!collections.isLoading?<p>{localeText("main", "ui.CollectionsWorkspace.1b79e2059be8df27")}</p>:null}</>:null}
    {collections.errorMessage?<p role="alert">{nativeDisplayText(collections.errorMessage??'')}</p>:null}
  </section>;
}
