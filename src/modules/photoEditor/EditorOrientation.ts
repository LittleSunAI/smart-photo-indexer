import type {NormalizedPoint} from './EditorContracts';
export interface WorkingOrientation {rotation:0|1|2|3;mirrored:boolean}
export const identityOrientation:WorkingOrientation={rotation:0,mirrored:false};
export function orientPoint(p:NormalizedPoint,o:WorkingOrientation):NormalizedPoint {
 const x=o.mirrored?1-p.x:p.x,y=p.y;
 switch(o.rotation){case 1:return {x:1-y,y:x};case 2:return {x:1-x,y:1-y};case 3:return {x:y,y:1-x};default:return {x,y};}
}
export function inverseOrientation(o:WorkingOrientation):WorkingOrientation{return {rotation:(o.mirrored?o.rotation:(4-o.rotation)%4) as WorkingOrientation['rotation'],mirrored:o.mirrored};}
/** Compose a visible-canvas rotation/flip on the left of R(rotation) H(mirrored). */
export function changeOrientation(o:WorkingOrientation,action:'left'|'right'|'horizontal'|'vertical'):WorkingOrientation {
 if(action==='left'||action==='right')return {...o,rotation:((o.rotation+(action==='right'?1:3))%4) as WorkingOrientation['rotation']};
 return {rotation:((action==='vertical'?2:0)+4-o.rotation)%4 as WorkingOrientation['rotation'],mirrored:!o.mirrored};
}
export function orientationTransform(o:WorkingOrientation):string{return `rotate(${o.rotation*90}deg) scaleX(${o.mirrored?-1:1})`;}
