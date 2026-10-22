import type {EditorDraft} from './EditorContracts';
import type {MaskStroke} from './EditorGestureSurface';
import {reconcileDraftSelection} from './EditorDraftSelection';

/** Merge only the applied mask; absence in the authoritative snapshot means deletion. */
export function mergeMaskApplication(now:EditorDraft,captured:EditorDraft,saved:EditorDraft,id:string,strokes:MaskStroke[]):EditorDraft{
 const formValues={...now.formValues},latest=formValues.__maskStrokes as MaskStroke[]|undefined;
 if(formValues.__maskAnnotationId===id&&latest&&latest.length>=strokes.length&&latest.slice(0,strokes.length).every((stroke,i)=>stroke===strokes[i])){
  const rest=latest.slice(strokes.length);if(rest.length)formValues.__maskStrokes=rest;else{delete formValues.__maskStrokes;delete formValues.__maskAnnotationId;}
 }
 const changed=saved.annotations.find(a=>a.id===id),newer=now.annotations.find(a=>a.id===id);
 const annotations=now.annotations.filter(a=>a.id!==id);
 const deletedWhileApplying=captured.annotations.some(a=>a.id===id)&&!newer;
 if(changed&&!deletedWhileApplying)annotations.push({...changed,...(newer?{instruction:newer.instruction,visible:newer.visible,included:newer.included}:{})});
 // Reconciliation rebuilds arrays even when selection IDs have not changed.
 // Preserve a genuinely different selection while accepting an equivalent set.
 const currentMasks=new Set(now.selectedMaskIds),capturedMasks=new Set(captured.selectedMaskIds);
 const selectionUnchanged=now.selectedAnnotationId===captured.selectedAnnotationId&&currentMasks.size===capturedMasks.size&&[...currentMasks].every(id=>capturedMasks.has(id));
 return reconcileDraftSelection({...now,annotations,formValues,
  editHistory:now.editHistory===captured.editHistory?saved.editHistory:now.editHistory,
  selectedAnnotationId:changed&&selectionUnchanged?id:now.selectedAnnotationId,
  selectedMaskIds:changed&&selectionUnchanged?[...new Set([...now.selectedMaskIds,id])]:now.selectedMaskIds});
}
