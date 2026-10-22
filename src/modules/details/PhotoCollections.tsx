import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useHelpPresentation,useTourDisclosure} from '../help/HelpPresentation';
import {Hint} from '../../shared/help/Hint';
import {useEffect,useRef,useState} from 'react';
import {useAppActions,useAppSelector} from '../../app/state/AppStateContext';
import {loadUiPreferences,saveUiPreferences,UI_PREFERENCES_EVENT} from '../../shared/UiPreferences';
import {Icon} from '../../shared/ui/Icon';
import type {ImageItem} from '../explorer/types';
import type {CollectionMutationResult} from '../collections/types';

export function PhotoCollections({items}:{items:ImageItem[]}){
  useCurrentLocale();
  const help=useHelpPresentation(),disclosure=useTourDisclosure('selection-collections');
 const actions=useAppActions(),state=useAppSelector(state=>state.collections);
 const collections=state.items,selectedCollections=[...new Set(items.flatMap(item=>item.collections??[]))];
 const [target,setTarget]=useState(()=>loadUiPreferences().collectionTarget??''),[name,setName]=useState(''),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const createdTarget=useRef<string|null>(null);
 useEffect(()=>{const sync=()=>setTarget(loadUiPreferences().collectionTarget??'');window.addEventListener(UI_PREFERENCES_EVENT,sync);return()=>window.removeEventListener(UI_PREFERENCES_EVENT,sync);},[]);
 // A loading/failed collection list is not evidence that a remembered collection was deleted.
 useEffect(()=>{if(help.open)return;if(collections.some(c=>c.id===createdTarget.current))createdTarget.current=null;if(target&&target!==createdTarget.current&&!state.isLoading&&!state.errorMessage&&state.lastLoadedAtUnix!==null&&!collections.some(c=>c.id===target)){setTarget('');saveUiPreferences({collectionTarget:null});}},[target,collections,state.isLoading,state.errorMessage,state.lastLoadedAtUnix,help.open]);
 const choose=(id:string)=>{saveUiPreferences({collectionTarget:id||null});setTarget(id);};
 const run=async(work:()=>Promise<CollectionMutationResult|null|void>,success?:(result:CollectionMutationResult)=>void)=>{if(busy)return;setBusy(true);setError('');try{const result=await work();if(result===null){setError(localeText("details", "ui.PhotoCollections.8e7e299920af1abf"));return;}if(result)success?.(result);}catch(e){setError(String(e));}finally{setBusy(false);}};
 const validTarget=collections.some(c=>c.id===target)?target:'';
 return <details {...disclosure} data-tour-id="main.details.collections" className="photo-collections details-collapsible-card" data-help-id="selection-collections"><summary>{localeText("details", "ui.PhotoCollections.436812a5b23769f8")}<Icon name="down"/></summary><div className="details-collapsible-card__body">
  <div className="collection-picker__row"><Hint message={{id:'collection.target'}}><select data-tour-id="main.details.collection-add" aria-label={localeText("details", "ui.PhotoCollections.5cb9eb27c85d76b8")} value={validTarget} disabled={busy||state.isLoading} onChange={e=>choose(e.target.value)}><option value="">{localeText("details", "ui.PhotoCollections.e8727f6bd1b85324")}</option>{collections.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</select></Hint><Hint message={{id:'collection.add'}}><button type="button" className="secondary-action" disabled={busy||!validTarget||!items.length} onClick={()=>void run(()=>actions.addSelectedImagesToCollection(validTarget))}>{localeText("details", "ui.PhotoCollections.af429e08e1eb48e9")}</button></Hint></div>
  <div className="collection-picker__row"><Hint message={{id:'collection.name'}}><input aria-label={localeText("details", "ui.PhotoCollections.e2a1fae2a3c97341")} placeholder={localeText("details", "ui.PhotoCollections.9141a9827431d28e")} value={name} maxLength={160} disabled={busy} onChange={e=>setName(e.target.value)}/></Hint><Hint message={{id:'collection.create'}}><button data-tour-id="main.details.collection-create" type="button" className="secondary-action collection-picker__create" aria-label={localeText("details", "ui.PhotoCollections.2ca976014beb4e88")} disabled={busy||!name.trim()} onClick={()=>void run(()=>actions.createCollection(name.trim()),result=>{if(result.collection){createdTarget.current=result.collection.id;choose(result.collection.id);setName('');}})}><Icon name="plus"/></button></Hint></div>
  {selectedCollections.length?<div className="tag-list">{selectedCollections.map(label=>{const collection=collections.find(c=>c.name===label);return <span key={label} className="tag-chip">{label}{collection?<Hint message={{id:'collection.remove',args:{название:label}}}><button type="button" disabled={busy} aria-label={localeText("details", "ui.PhotoCollections.57619cf26874f28e", {"value1": (label)})} onClick={()=>void run(()=>actions.removeSelectedFromCollection(collection.id))}><Icon name="close"/></button></Hint>:null}</span>;})}</div>:null}
  {error||state.errorMessage?<p role="alert">{error||state.errorMessage}</p>:null}
 </div></details>;
}
