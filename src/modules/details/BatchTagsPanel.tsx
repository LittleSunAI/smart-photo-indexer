import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useTourDisclosure} from '../help/HelpPresentation';
import {Hint} from "../../shared/help/Hint";
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';
import { useEffect,useRef,useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import type { ExplorerItem } from '../explorer/types';
import type { IndexMetadataUpdateResult } from '../index/types';
import { useAppActions, useAppDispatch, useAppSelector } from '../../app/state/AppStateContext';
import {Icon} from '../../shared/ui/Icon';
import {isQueryView} from '../../app/state/QuickSearch';

interface Preview{planId:number;images:Array<{imageId:string;before:string[];after:string[]}>}
interface TagSummary{imageIds:string[];tags:string[];selection:string}
interface XmpImportResult{updatedImages:number;unchangedImages:number;conflicts:number;failedFiles:number;updates:IndexMetadataUpdateResult[];items:Array<{paths:string[];error?:string;descriptionConflict?:boolean}>}
export function BatchTagsPanel({items}:{items:ExplorerItem[]}){
  useCurrentLocale();
  const batchDisclosure=useTourDisclosure('batch-tags'),xmpDisclosure=useTourDisclosure('xmp-metadata');
 const actions=useAppActions();
 const snapshot=useAppSelector(state=>isQueryView(state)?state.search.selectionSnapshot:undefined);
 const count=snapshot?snapshot.totalCount-snapshot.excludedPaths.length:items.filter(item=>item.kind==='image').length;
 const ids=[...new Set(items.filter(item=>item.kind==='image').map(item=>item.indexedImageId).filter((id):id is string=>Boolean(id)))];
 const selection=snapshot?JSON.stringify([snapshot.input,snapshot.excludedPaths]):ids.slice().sort().join('\n');const generation=useRef(0);const undoGeneration=useRef(0);const dispatch=useAppDispatch();
 const currentSelection=useRef(selection);currentSelection.current=selection;
 const mounted=useRef(true),operation=useRef(false);
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;};},[]);
 const [add,setAdd]=useState(''),[remove,setRemove]=useState<string[]>([]),[summary,setSummary]=useState<TagSummary|null>(null),[refresh,setRefresh]=useState(0),[loading,setLoading]=useState(false),[undoId,setUndoId]=useState<number|null>(null),[busy,setBusy]=useState(false),[message,setMessage]=useState('');
 const [xmpReport,setXmpReport]=useState<XmpImportResult|null>(null);
 const tagVersions=items.filter(i=>i.kind==='image').map(i=>`${i.indexedImageId}:${i.tagsVersion}:${i.tags.join('\n')}`).join('|');
 useEffect(()=>{let disposed=false;const request=++undoGeneration.current;void invoke<number|null>('get_last_batch_tags_action').then(id=>{if(!disposed && request===undoGeneration.current)setUndoId(id);}).catch(()=>{});return ()=>{disposed=true;};},[]);
 useEffect(()=>{setAdd('');setRemove([]);setMessage('');setXmpReport(null);},[selection]);
 useEffect(()=>{let disposed=false;const request=++generation.current;setSummary(null);if(!count){setLoading(false);return;}setLoading(true);void(async()=>{
  try{const imageIds=snapshot?await invoke<string[]>('get_search_selection_image_ids',{input:snapshot.input,excludedPaths:snapshot.excludedPaths,maximum:256}):ids;
   const result=await invoke<Omit<TagSummary,'selection'>>('get_batch_tag_summary',{imageIds});if(!disposed&&request===generation.current)setSummary({...result,selection});
  }catch(error){if(!disposed&&request===generation.current)setMessage(String(error));}finally{if(!disposed&&request===generation.current)setLoading(false);}
 })();return()=>{disposed=true;};},[selection,refresh,tagVersions]);
 const tags=(value:string)=>value.split(/[,;\n]/).map(tag=>tag.trim()).filter(Boolean);
 async function commit(undo:boolean){if(operation.current||!undo&&summary?.selection!==selection||undo&&!undoId)return;operation.current=true;const captured=selection;undoGeneration.current++;setBusy(true);setMessage('');try{
  const planId=undo?undoId!:(await invoke<Preview>('preview_batch_tags',{input:{imageIds:summary!.imageIds,add:tags(add),remove}})).planId;
  // A changed selection before commit cancels this draft; an already accepted commit keeps its original IDs.
  if(!undo&&currentSelection.current!==captured)return;
  const results=await invoke<IndexMetadataUpdateResult[]>('commit_batch_tags',{planId,undo});for(const result of results)dispatch({type:'index/metadataUpdateSucceeded',result});
  if(mounted.current){setUndoId(undo?null:planId);if(currentSelection.current===captured){setAdd('');setRemove([]);setRefresh(v=>v+1);setMessage(localeText("details", "ui.BatchTagsPanel.f7e36f4b6426ea04", {"value1": (undo?localeText("details", "ui.BatchTagsPanel.bb73ed9196481f74"):localeText("details", "ui.BatchTagsPanel.aff47b3b9d99a592")), "value2": (results.length)}));}}
 }catch(error){if(mounted.current&&currentSelection.current===captured)setMessage(String(error));}finally{operation.current=false;if(mounted.current)setBusy(false);}}
 async function exportXmp(importing=false){if(operation.current)return;operation.current=true;const captured=selection;setBusy(true);setMessage('');setXmpReport(null);try{
  const selected=snapshot?await actions.resolveSelectedImages(257):items.filter(item=>item.kind==='image');
  if(selected.length>256)throw new Error(localeText("details", "ui.BatchTagsPanel.9038a6fa1bed1a6c"));
  const paths=selected.map(item=>item.path);
  if(currentSelection.current!==captured)return;
  if(importing){const result=await invoke<XmpImportResult>('import_image_xmp',{paths});for(const update of result.updates)dispatch({type:'index/metadataUpdateSucceeded',result:update});if(mounted.current&&currentSelection.current===captured){setXmpReport(result);setMessage(localeText("details", "ui.BatchTagsPanel.8aadc0093663000f", {"value1": (result.updatedImages), "value2": (result.unchangedImages), "value3": (result.conflicts), "value4": (result.failedFiles)}));setRefresh(v=>v+1);}}
  else{const result=await invoke<{saved:number;failed:number;items:Array<{error?:string}>}>('export_image_xmp',{paths});if(mounted.current&&currentSelection.current===captured)setMessage(localeText("details", "ui.BatchTagsPanel.a59fd4ea6e7389d9", {"value1": (result.saved), "value2": (result.failed), "value3": (result.items.find(item=>item.error)?.error || '')}));}
 }catch(error){if(mounted.current&&currentSelection.current===captured)setMessage(String(error));}finally{operation.current=false;if(mounted.current)setBusy(false);}}
  return <section className="batch-tags-section" aria-label={localeText("details", "ui.BatchTagsPanel.f888fed4cb337f16")}><details {...batchDisclosure} className="details-disclosure">
    <Hint message={{id:"details.batch"}}><summary data-tour-id="main.tags.batch-overview"><HintTitle title={localeText("details", "ui.BatchTagsPanel.0d9f57bc5f787169")}  message={{id:"details.batch"}}/><Icon name="down"/></summary></Hint><div className="details-disclosure__body"><p>{snapshot?localeText("details", "ui.BatchTagsPanel.8afe92fb9d320523", {"value1": (count)}):localeText("details", "ui.BatchTagsPanel.c9a0cc0211f53761", {"value1": (ids.length)})}</p>
   <details className="compact-help"><Hint message={{id:"common.expand",args:{раздел:localeText("details", "ui.BatchTagsPanel.a6cf4e33b58d2260")}}}><summary>{localeText("details", "ui.BatchTagsPanel.3e4ddb4ef5773e97")}</summary></Hint><p>{localeText("details", "ui.BatchTagsPanel.360d6fad711975a9")}</p></details>
  <div data-tour-id="main.tags.batch-add" className="batch-tags-add"><textarea aria-label={localeText("details", "ui.BatchTagsPanel.a071d99926b85208")} placeholder={localeText("details", "ui.BatchTagsPanel.8b7fccdc01dd0b83")} rows={6} value={add} disabled={busy||loading} onChange={event=>setAdd(event.target.value)}/></div>
  {loading?<p role="status">{localeText("details", "ui.BatchTagsPanel.ed745c1377e3271c")}</p>:null}
  {summary?.selection===selection?<div data-tour-id="main.tags.batch-remove" className="batch-tags-union tag-list" aria-label={localeText("details", "ui.BatchTagsPanel.b1b9fac881501557")}>{summary.tags.length?summary.tags.map(tag=><span key={tag} className={`tag-chip${remove.includes(tag)?' tag-chip--pending-remove':''}`}><span>{tag}</span><button type="button" disabled={busy} aria-label={localeText("details", "ui.BatchTagsPanel.d96d74519017cb07", {"value1": (remove.includes(tag)?localeText("details", "ui.BatchTagsPanel.c2f71ff38db11117"):localeText("details", "ui.BatchTagsPanel.7fc6528aff324e0b")), "value2": (tag)})} aria-pressed={remove.includes(tag)} onClick={()=>setRemove(values=>values.includes(tag)?values.filter(v=>v!==tag):[...values,tag])}><Icon name={remove.includes(tag)?'undo':'close'}/></button></span>):<span className="metadata-edit-note">{localeText("details", "ui.BatchTagsPanel.170bebb047fec21d")}</span>}</div>:null}
  <div className="batch-tags-actions"><Hint message={{id:'details.batch-apply'}}><button data-tour-id="main.tags.batch-save" type="button" className="primary-action" disabled={busy||loading||summary?.selection!==selection||!tags(add).length&&!remove.length} onClick={()=>void commit(false)}>{localeText("details", "ui.BatchTagsPanel.4cbf86794cc4f05d")}</button></Hint></div>
  {undoId?<Hint message={{id:"details.batch-undo"}}><button data-tour-id="main.tags.batch-save" className="secondary-action" disabled={busy} onClick={()=>{void commit(true);}}>{localeText("details", "ui.BatchTagsPanel.fc79ff52834339c7")}</button></Hint>:null}
   </div></details><details {...xmpDisclosure} className="details-disclosure" data-help-id="xmp-metadata"><Hint message={{id:"details.xmp"}}><summary data-tour-id="main.tags.xmp"><HintTitle title={localeText("details", "ui.BatchTagsPanel.8de85e9471ac9a13")}  message={{id:"details.xmp"}}/><Icon name="down"/></summary></Hint><div className="details-disclosure__body">
   <p>{localeText("details", "ui.BatchTagsPanel.964d6105bf2a3a6e")}</p>
   <div className="xmp-metadata-actions"><Hint message={{id:"details.xmp-export"}}><button className="secondary-action" disabled={busy||!count||count>256} onClick={()=>void exportXmp()}>{localeText("details", "ui.BatchTagsPanel.1775715432ce3d5e")}</button></Hint><Hint message={{id:"details.xmp-import"}}><button className="secondary-action" disabled={busy||!count||count>256} onClick={()=>void exportXmp(true)}>{localeText("details", "ui.BatchTagsPanel.2fbfab0f21924eeb")}</button></Hint></div>
   {xmpReport&&(xmpReport.conflicts||xmpReport.failedFiles)?<details className="xmp-import-report"><summary>{localeText("details", "ui.BatchTagsPanel.b1d2fd47f2714b9e")}</summary>{xmpReport.items.filter(item=>item.error||item.descriptionConflict).map((item,i)=><p key={i} title={item.paths.join('\n')}>{item.paths.map(path=>path.split(/[\\/]/).at(-1)).join(', ')}: {item.error??localeText("details", "ui.BatchTagsPanel.cb5bca311a60892f")}</p>)}</details>:null}
   </div></details>{message?<p role="status">{message}</p>:null}
 </section>;
}
