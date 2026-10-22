import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {createContext,useContext,useEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {convertFileSrc} from '@tauri-apps/api/core';
import type {EditorAsset,EditorDraft} from './EditorContracts';
import {editorService} from './EditorService';
import {currentInstruction,localInstructions} from './EditorInstructionState';
import {Icon} from '../../shared/ui/Icon';
import {referenceDropHandlers} from './EditorReferenceDrag';
import {supportsImageReferences,showsImageReferences} from './EditorReferenceTarget';
import {EditorHint} from './EditorHint';

interface ReferenceContext {draft:EditorDraft;epoch:string;assets:EditorAsset[];busy:boolean;capacity?:{used:number;max:number;remaining:number}|null;save():Promise<void>;drop(target:string,stepId:string,replace:boolean):Promise<void>;attach(target:string,replace:boolean,paths?:string[],assetIds?:string[]):Promise<void>;notify(message:string):void;}
export const EditorReferencesContext=createContext<ReferenceContext|null>(null);
export function referenceCount(d:EditorDraft,extraTarget?:string,assets:EditorAsset[]=[],capacity=false):number{
 const op=currentInstruction(d),ids=localInstructions(d.operation)?d.annotations.filter(a=>a.included&&['point','rect'].includes(a.kind)&&(capacity||op.texts[a.id]?.trim())).map(a=>a.id):[];
 const selected=new Set([...ids,...(extraTarget?[extraTarget]:[])]),inputs=new Set<string>();const content=(id:string)=>assets.find(a=>a.id===id)?.sha256??id;
 if(d.orientation.rotation===0&&!d.orientation.mirrored)inputs.add(content(d.assetId));
 for(const r of d.references)inputs.add(content(r.assetId));
 for(const a of d.annotations){if(!selected.has(a.id)||!a.included&&a.id!==extraTarget)continue;for(const r of d.annotationReferences[a.id]??[]){if(r.included)inputs.add(content(r.assetId));}}
 return inputs.size+(d.orientation.rotation!==0||d.orientation.mirrored?1:0);
}

export function EditorImageReferenceCapsule(){
  useCurrentLocale();
 const context=useContext(EditorReferencesContext);
 if(!context||!showsImageReferences(context.draft))return null;
 const d=context.draft,capacity=context.capacity;
 return <div data-tour-id="editor.references.whole" className="editor-image-reference-capsule" data-reference-target="image" {...referenceDropHandlers(context,'image')}>
  <div className="editor-section-heading editor-image-reference-heading"><strong>{localeText("editor", "ui.EditorReferenceControls.49751fcb424c2315")}{' '}{d.references.length}</strong><EditorHint label={localeText("editor", "ui.EditorReferenceControls.7e66c5e69b7561ec")} message={{id:"reference.whole"}}>{localeText("editor", "ui.EditorReferenceControls.47be3b96f802c7cd")}</EditorHint><small data-tour-id="editor.references.limits" className="editor-image-reference-count" title={localeText("editor", "ui.EditorReferenceControls.1c1d431a5cadbefa")}>{capacity?`${capacity.used}/${capacity.max}`:''}</small><Hint message={capacity?.remaining===0?{id:'reference.limit',args:{лимит:capacity.max}}:{id:!capacity?'reference.wait-model':'reference.add-whole'}}><button type="button" className="editor-image-reference-add" aria-label={localeText("editor", "ui.EditorReferenceControls.7d3d58bdc7db7ba3")} title={capacity?.remaining===0?localeText("editor", "ui.EditorReferenceControls.4540e82877dde48f"):!capacity?localeText("editor", "ui.EditorReferenceControls.81f9163ccda64d2a"):localeText("editor", "ui.EditorReferenceControls.36e90bfa24167f11")} disabled={context.busy||!capacity||capacity.remaining===0} onClick={e=>{e.preventDefault();e.stopPropagation();void context.attach('image',false).catch(error=>context.notify(String(error)));}}><Icon name="plus"/></button></Hint></div>
  {d.references.length?<EditorReferenceControls targetId="image" hideAdd/>:null}
 </div>;
}
export function EditorReferenceControls({targetId,showStored=false,hideAdd=false}:{targetId?:string;showStored?:boolean;hideAdd?:boolean}){
  useCurrentLocale();
 const context=useContext(EditorReferencesContext),[capacity,setCapacity]=useState<{used:number;max:number;remaining:number}|null>(null),[reason,setReason]=useState('');
 const d=context?.draft,op=d?currentInstruction(d):null,target=targetId??op?.target,general=target==='image';
 const mark=d?.annotations.find(a=>a.id===target),items=general?d?.references.map(r=>({...r,included:true}))??[]:d?.annotationReferences[target??'']??[];
 const identity=d?JSON.stringify({model:d.modelId,operation:d.operation,orientation:d.orientation,base:d.assetId,refs:d.references,bindings:d.annotationReferences,targets:op?.mode==='multi'?op.enabled:[op?.target],included:d.annotations.map(a=>[a.id,a.included]),target}):'';
 useEffect(()=>{
  setCapacity(null);setReason('');
  if(!context||!target||general&&!supportsImageReferences(context.draft)||!general&&(!mark||mark.kind==='mask'||!localInstructions(context.draft.operation)))return;
  let disposed=false;const timer=setTimeout(()=>void editorService.capacity(context.epoch,context.draft,target).then(v=>{if(!disposed)setCapacity(v);}).catch(e=>{if(!disposed)setReason(String(e));}),80);
  return()=>{disposed=true;clearTimeout(timer);};
 },[identity,context?.epoch]);
 if(!context||!d||!target||general&&!supportsImageReferences(d)||!general&&(!mark||mark.kind==='mask'||!showStored&&!localInstructions(d.operation)))return null;
 const inactive=general?!supportsImageReferences(d):!localInstructions(d.operation),full=!capacity||capacity.remaining===0,point=mark?.kind==='point',disabled=context.busy||inactive||full||point&&items.length>0;
 const hint=context.busy?localeText("editor", "ui.EditorReferenceControls.820fccf41ef49887"):inactive?localeText("editor", "ui.EditorReferenceControls.2a938cfefdeda58d"):reason||(!capacity?localeText("editor", "ui.EditorReferenceControls.af96536de24bc408"):capacity.remaining===0?localeText("editor", "ui.EditorReferenceControls.6e240059e59f5935", {"value1": (capacity.used), "value2": (capacity.max)}):point&&items.length?localeText("editor", "ui.EditorReferenceControls.e3d6a45788576cbd"):localeText("editor", "ui.EditorReferenceControls.32af7cdc79d2c486", {"value1": (capacity.remaining)}));
 return <div data-tour-id={target==='image'?undefined:'editor.references.mark'} className="editor-reference-controls" data-reference-target={target} {...referenceDropHandlers(context,target)} onPaste={e=>{
  const paths=e.clipboardData.getData('text/plain').split(/\r?\n/).map(p=>p.trim().replace(/^"|"$/g,'')).filter(p=>/^[A-Za-z]:\\/.test(p));if(!paths.length)return;
  e.preventDefault();e.stopPropagation();if(disabled)context.notify(hint);else void context.attach(target,false,paths).catch(error=>context.notify(String(error)));
 }}>
  <div className="editor-reference-thumbnails">
   {!hideAdd?<span className="editor-reference-add" tabIndex={0} title={point&&items.length&&!context.busy&&!inactive?localeText("editor", "ui.EditorReferenceControls.215a3b630779d2d6"):hint}><Hint message={{id:point&&items.length?'reference.replace-point':'reference.add-mark'}} supplement={inactive?{id:'reference.unsupported'}:point&&items.length?{id:'reference.point-replace'}:undefined}><button type="button" aria-label={point&&items.length?localeText("editor", "ui.EditorReferenceControls.7a794de46b51c1d8"):localeText("editor", "ui.EditorReferenceControls.ff96e303e5164077")} disabled={point&&items.length?context.busy||inactive||Boolean(reason):disabled} onClick={()=>void context.attach(target,point&&items.length>0).catch(()=>{})}><Icon name={point&&items.length?'refresh':'plus'}/></button></Hint></span>:null}
   {items.map((r,index)=>{const a=context.assets.find(a=>a.id===r.assetId);return <figure key={r.assetId} className={r.included?'':'is-excluded'}>
    {a?<ReferenceThumbnail asset={a}/>:null}
    {!general?<Hint message={{id:"reference.include"}}><label title={localeText("editor", "ui.EditorReferenceControls.6a1c3ff526d1983a")}><input type="checkbox" aria-label={localeText("editor", "ui.EditorReferenceControls.d8eccfe0c956e810")} disabled={context.busy} checked={r.included} onChange={e=>window.dispatchEvent(new CustomEvent('editor-reference-change',{detail:{target,assetId:r.assetId,included:e.target.checked}}))}/></label></Hint>:null}
    {!point?<div className="editor-reference-order"><Hint message={{id:"reference.order"}}><button type="button" aria-label={general?localeText("editor", "ui.EditorReferenceControls.b7ba91ad6fe9a1e1"):localeText("editor", "ui.EditorReferenceControls.37100ad3e701f7b4")} disabled={context.busy||index===0} onClick={()=>window.dispatchEvent(new CustomEvent('editor-reference-change',{detail:{target,assetId:r.assetId,move:-1}}))}>‹</button></Hint><Hint message={{id:"reference.order"}}><button type="button" aria-label={general?localeText("editor", "ui.EditorReferenceControls.db837dd8826c3631"):localeText("editor", "ui.EditorReferenceControls.a64862fa0d8277b9")} disabled={context.busy||index===items.length-1} onClick={()=>window.dispatchEvent(new CustomEvent('editor-reference-change',{detail:{target,assetId:r.assetId,move:1}}))}>›</button></Hint></div>:null}
    <Hint message={{id:"reference.detach"}}><button type="button" title={localeText("editor", "ui.EditorReferenceControls.769e710aef358993")} aria-label={localeText("editor", "ui.EditorReferenceControls.3d944645494d2782")} disabled={context.busy} onClick={()=>window.dispatchEvent(new CustomEvent('editor-reference-change',{detail:{target,assetId:r.assetId,remove:true}}))}><Icon name="close"/></button></Hint>
   </figure>;})}
  </div>
 </div>;
}

export function ReferenceThumbnail({asset}:{asset:EditorAsset}){
  useCurrentLocale();
 const [open,setOpen]=useState(false),timer=useRef<ReturnType<typeof setTimeout>|null>(null),anchor=useRef<HTMLImageElement>(null);
 useEffect(()=>()=>{if(timer.current)clearTimeout(timer.current);},[]);
 const show=()=>{timer.current=setTimeout(()=>setOpen(true),650);},hide=()=>{if(timer.current)clearTimeout(timer.current);setOpen(false);};const box=anchor.current?.getBoundingClientRect();
 return <><img ref={anchor} tabIndex={0} draggable={false} src={convertFileSrc(asset.previewPath)} alt={localeText("editor", "ui.EditorReferenceControls.a03c8e05b1b378df")} onMouseEnter={show} onMouseLeave={hide} onFocus={()=>setOpen(true)} onBlur={hide}/>{open&&box?createPortal(<div role="tooltip" className="editor-reference-preview" style={{left:Math.max(8,Math.min(innerWidth-208,box.left-200)),top:Math.max(8,Math.min(innerHeight-180,box.top))}}><img src={convertFileSrc(asset.previewPath)} alt={localeText("editor", "ui.EditorReferenceControls.204823e16f617e29")}/><span>{asset.width} × {asset.height}</span></div>,document.body):null}</>;
}
