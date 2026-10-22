import type {EditorDraft} from './EditorContracts';

/** Keep selection references consistent without changing the user's existing masks. */
export function reconcileDraftSelection(d:EditorDraft):EditorDraft {
  const ids=new Set(d.annotations.map(a=>a.id));
  const masks=new Set(d.annotations.filter(a=>a.kind==='mask').map(a=>a.id));
  return {...d,selectedAnnotationId:d.selectedAnnotationId&&ids.has(d.selectedAnnotationId)?d.selectedAnnotationId:null,
    selectedMaskIds:[...new Set(d.selectedMaskIds)].filter(id=>masks.has(id))};
}
