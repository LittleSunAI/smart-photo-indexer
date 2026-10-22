import type {EditorDraft} from './EditorContracts';
import {currentInstruction,localInstructions} from './EditorInstructionState';

export const supportsImageReferences=(d:EditorDraft)=>['universal-edit','replace-object','compose'].includes(d.operation);
export function showsImageReferences(d:EditorDraft):boolean{
 if(!supportsImageReferences(d))return false;
 if(!localInstructions(d.operation))return true;
 return currentInstruction(d).target==='image'&&d.selectedMaskIds.length===0&&d.annotations.find(a=>a.id===d.selectedAnnotationId)?.kind!=='mask';
}
export function canAttachReference(d:EditorDraft,target:string):boolean{
 if(target==='image')return supportsImageReferences(d);
 return localInstructions(d.operation)&&d.annotations.some(a=>a.id===target&&(a.kind==='point'||a.kind==='rect'));
}
