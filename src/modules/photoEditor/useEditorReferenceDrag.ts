import {useEffect,useRef,type PointerEvent as ReactPointerEvent,type MouseEvent as ReactMouseEvent} from 'react';
import type {EditorDraft} from './EditorContracts';
import {canAttachReference} from './EditorReferenceTarget';

interface Context {epoch:string;projectId:string;draft:EditorDraft;busy:boolean;stepIds:string[];drop(target:string,stepId:string,replace:boolean):Promise<void>;notify(message:string):void}
interface Gesture {pointerId:number;stepId:string;epoch:string;projectId:string;x:number;y:number;moving:boolean;source:HTMLElement;target:HTMLElement|null}

/** Internal pointer delivery coexists with Tauri's native external-file drop handler. */
export function useEditorReferenceDrag(context:Context){
 const latest=useRef(context);latest.current=context;
 const gesture=useRef<Gesture|null>(null),suppress=useRef<string|null>(null);
 const clear=()=>{const g=gesture.current;gesture.current=null;if(g){g.target?.classList.remove('is-drop-target');g.source.classList.remove('is-reference-dragging');if(g.source.hasPointerCapture(g.pointerId))g.source.releasePointerCapture(g.pointerId);}};
 useEffect(()=>{
  const cancel=()=>{const g=gesture.current;if(g?.moving)suppress.current=g.stepId;clear();};
  const escape=(e:KeyboardEvent)=>{if(e.key==='Escape'&&gesture.current){e.preventDefault();e.stopImmediatePropagation();cancel();}};
  window.addEventListener('blur',cancel);window.addEventListener('keydown',escape,true);
  return()=>{window.removeEventListener('blur',cancel);window.removeEventListener('keydown',escape,true);clear();};
 },[]);
 useEffect(()=>{clear();suppress.current=null;},[context.epoch,context.projectId]);
 const begin=(e:ReactPointerEvent<HTMLButtonElement>,stepId:string)=>{
  suppress.current=null;if(e.button!==0||e.ctrlKey||e.metaKey||e.shiftKey||latest.current.busy)return;
  clear();gesture.current={pointerId:e.pointerId,stepId,epoch:context.epoch,projectId:context.projectId,x:e.clientX,y:e.clientY,moving:false,source:e.currentTarget,target:null};
  e.currentTarget.setPointerCapture(e.pointerId);
 };
 const move=(e:ReactPointerEvent<HTMLButtonElement>)=>{
  const g=gesture.current,c=latest.current;if(!g||g.pointerId!==e.pointerId)return;
  if(g.epoch!==c.epoch||g.projectId!==c.projectId||c.busy){suppress.current=g.stepId;clear();return;}
  if(!g.moving&&Math.hypot(e.clientX-g.x,e.clientY-g.y)<5)return;
  g.moving=true;g.source.classList.add('is-reference-dragging');e.preventDefault();
  const candidate=document.elementFromPoint(e.clientX,e.clientY)?.closest<HTMLElement>('[data-reference-target]')??null;
  const target=candidate?.dataset.referenceTarget&&canAttachReference(c.draft,candidate.dataset.referenceTarget)?candidate:null;
  if(target!==g.target){g.target?.classList.remove('is-drop-target');g.target=target;target?.classList.add('is-drop-target');}
 };
 const end=(e:ReactPointerEvent<HTMLButtonElement>)=>{
  const g=gesture.current,c=latest.current;if(!g||g.pointerId!==e.pointerId)return;
  if(g.moving){e.preventDefault();suppress.current=g.stepId;}
  // Resolve the actual release point, not a stale hovered node after a scroll/disclosure.
  const hit=document.elementFromPoint(e.clientX,e.clientY)?.closest<HTMLElement>('[data-reference-target]'),target=hit?.dataset.referenceTarget;
  const mark=c.draft.annotations.find(a=>a.id===target),allowed=g.moving&&g.epoch===c.epoch&&g.projectId===c.projectId&&!c.busy&&c.stepIds.includes(g.stepId)&&Boolean(target&&canAttachReference(c.draft,target));
  clear();if(allowed&&target)void c.drop(target,g.stepId,mark?.kind==='point'&&Boolean(c.draft.annotationReferences[target]?.length)).catch(error=>c.notify(String(error)));
 };
 return {begin,move,end,cancel:()=>{const g=gesture.current;if(g?.moving)suppress.current=g.stepId;clear();},click:(e:ReactMouseEvent,stepId:string)=>{if(suppress.current===stepId){suppress.current=null;e.preventDefault();e.stopPropagation();return true;}return false;}};
}
