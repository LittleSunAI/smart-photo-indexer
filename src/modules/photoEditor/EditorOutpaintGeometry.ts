import type {EditorAsset,EditorDraft} from './EditorContracts';
import {inverseOrientation,orientPoint} from './EditorOrientation';

export interface OutpaintRect {version:1;x:number;y:number;width:number;height:number}
type AssetSize=Pick<EditorAsset,'width'|'height'>;
export function outpaintFrameDraft(d:EditorDraft,rect:OutpaintRect):EditorDraft{
 const values={...d.formValues};for(const side of ['Left','Right','Top','Bottom'])delete values['__outpaint'+side];
 return {...d,formValues:{...values,__outpaintFrame:rect}};
}
export function outpaintGeometry(d:EditorDraft,asset:AssetSize){
 const w=d.orientation.rotation%2?asset.height:asset.width,h=d.orientation.rotation%2?asset.width:asset.height,padding=Math.max(0,512-h);
 const raw=d.formValues.__outpaintFrame as OutpaintRect|undefined;
 const defaults=[128,128,Math.floor(padding/2),Math.ceil(padding/2)];
 const sides=['Left','Right','Top','Bottom'].map((side,i)=>Number(d.formValues['__outpaint'+side]??defaults[i]));
 if(!raw&&sides.some(v=>!Number.isSafeInteger(v)||v<0||v>1024))return null;
 const [l,r,t,b]=sides,rect:OutpaintRect=raw??{version:1,x:-l,y:-t,width:w+l+r,height:h+t+b};
 if(rect.version!==1||[rect.x,rect.y,rect.width,rect.height].some(v=>!Number.isSafeInteger(v))||rect.width<1||rect.height<1||Math.abs(rect.x)>32000000||Math.abs(rect.y)>32000000||rect.width>32000000||rect.height>32000000)return null;
 const {x,y,width,height}=rect,left=-x,right=x+width-w,top=-y,bottom=y+height-h;
 const cx=Math.max(0,x),cy=Math.max(0,y),cw=Math.max(0,Math.min(w,x+width)-cx),ch=Math.max(0,Math.min(h,y+height)-cy);
 const requiresCrop=cx>0||cy>0||cw<w||ch<h,hasExpansion=left>0||right>0||top>0||bottom>0;
 const bounded=width<=2048&&height<=2048&&Math.max(left,right,top,bottom)<=1024&&cw>0&&ch>0;
 return {rect,w,h,x,y,left,right,top,bottom,width,height,crop:{x:cx,y:cy,width:cw,height:ch},requiresCrop,hasExpansion,bounded,valid:bounded&&!requiresCrop&&hasExpansion&&width>=512&&height>=512};
}
export function centeredOutpaintSize(d:EditorDraft,asset:AssetSize,width:number,height:number){
 const g=outpaintGeometry(d,asset);if(!g)return d;
 return outpaintFrameDraft(d,{version:1,x:Math.floor((g.w-width)/2),y:Math.floor((g.h-height)/2),width,height});
}
/** R lives in working coordinates; transform its corners once when orientation changes. */
export function orientOutpaint(previous:EditorDraft,next:EditorDraft,asset:AssetSize):EditorDraft{
 if(previous.operation!=='outpaint'&&!previous.formValues.__outpaintFrame&&!Object.keys(previous.formValues).some(k=>/^__outpaint(Left|Right|Top|Bottom)$/.test(k)))return next;
 const g=outpaintGeometry(previous,asset);if(!g)return next;
 const nw=next.orientation.rotation%2?asset.height:asset.width,nh=next.orientation.rotation%2?asset.width:asset.height;
 const points=[[g.x,g.y],[g.x+g.width,g.y],[g.x,g.y+g.height],[g.x+g.width,g.y+g.height]].map(([x,y])=>{const canonical=orientPoint({x:x/g.w,y:y/g.h},inverseOrientation(previous.orientation));const p=orientPoint(canonical,next.orientation);return {x:Math.round(p.x*nw),y:Math.round(p.y*nh)};});
 const x=Math.min(...points.map(p=>p.x)),y=Math.min(...points.map(p=>p.y));
 return outpaintFrameDraft(next,{version:1,x,y,width:Math.max(...points.map(p=>p.x))-x,height:Math.max(...points.map(p=>p.y))-y});
}
