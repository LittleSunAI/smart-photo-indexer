import {text as localeText} from "../../shared/i18n/Runtime";
import type {DragEvent} from 'react';
import type {EditorDraft} from './EditorContracts';
import {canAttachReference} from './EditorReferenceTarget';

export const EDITOR_REFERENCE_DRAG_TYPE='application/x-smart-photo-reference';
interface DropContext {
 draft:EditorDraft;epoch:string;busy:boolean;
 drop(target:string,stepId:string,replace:boolean):Promise<void>;
 notify(message:string):void;
}
/** Internal drags carry immutable frame IDs, never filesystem paths or image bytes. */
export function referenceDropHandlers(context:DropContext,target:string){
 const isReference=(e:DragEvent<HTMLElement>)=>e.dataTransfer.types.includes(EDITOR_REFERENCE_DRAG_TYPE);
 const permitted=()=>!context.busy&&canAttachReference(context.draft,target);
 return {
  onDragOver(e:DragEvent<HTMLElement>){if(!isReference(e))return;e.preventDefault();e.stopPropagation();e.dataTransfer.dropEffect=permitted()?'copy':'none';if(permitted())e.currentTarget.classList.add('is-drop-target');},
  onDragLeave(e:DragEvent<HTMLElement>){if(!e.currentTarget.contains(e.relatedTarget as Node|null))e.currentTarget.classList.remove('is-drop-target');},
  onDrop(e:DragEvent<HTMLElement>){
   if(!isReference(e))return;e.preventDefault();e.stopPropagation();e.currentTarget.classList.remove('is-drop-target');
   if(!permitted()){context.notify(localeText("editor", "ui.EditorReferenceDrag.5b6b99bf3a88c242"));return;}
   try{
    const value=JSON.parse(e.dataTransfer.getData(EDITOR_REFERENCE_DRAG_TYPE));
    if(value.epoch!==context.epoch||typeof value.stepId!=='string'||!value.stepId||value.stepId.length>160)throw Error(localeText("editor", "ui.EditorReferenceDrag.a407f5bbb1dde7d6"));
    const point=context.draft.annotations.find(a=>a.id===target)?.kind==='point';
    void context.drop(target,value.stepId,point&&Boolean(context.draft.annotationReferences[target]?.length)).catch(()=>{});
   }catch(e){context.notify(String(e));}
  },
 };
}
