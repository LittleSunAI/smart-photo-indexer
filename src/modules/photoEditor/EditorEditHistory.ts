import type {EditorAnnotation} from './EditorContracts';
import type {WorkingOrientation} from './EditorOrientation';
import type {MaskStroke} from './EditorGestureSurface';
export interface EditFrame {
 stepId:string|null;assetId:string;orientation:WorkingOrientation|null;
 annotationReferences?:Record<string,Array<{assetId:string;included:boolean}>>;annotations:EditorAnnotation[];maskStrokes:MaskStroke[];maskAnnotationId:string|null;selectedMaskIds?:string[];instructionContext?:unknown;
}
export interface EditHistory {undo:EditFrame[];redo:EditFrame[]}
export const emptyEditHistory=():EditHistory=>({undo:[],redo:[]});
export function limitHistory(h:EditHistory):EditHistory {
 const next={undo:[...h.undo],redo:[...h.redo]};
 while(next.undo.length+next.redo.length>50||new TextEncoder().encode(JSON.stringify(next)).length>256*1024){
  if(next.undo.length)next.undo.shift();else if(next.redo.length)next.redo.shift();else break;
 }
 return next;
}
export function recordEdit(h:EditHistory,previous:EditFrame):EditHistory{return limitHistory({undo:[...h.undo,structuredClone(previous)],redo:[]});}
export function stepEdit(h:EditHistory,current:EditFrame,redo=false):{history:EditHistory;frame:EditFrame}|null {
 const from=redo?h.redo:h.undo,frame=from.at(-1);if(!frame||frame.assetId!==current.assetId||(frame.stepId!==null&&frame.stepId!==current.stepId))return null;
 return {frame:structuredClone(frame),history:limitHistory(redo?{undo:[...h.undo,structuredClone(current)],redo:h.redo.slice(0,-1)}:{undo:h.undo.slice(0,-1),redo:[...h.redo,structuredClone(current)]})};
}
