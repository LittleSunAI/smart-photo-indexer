import type {TransformFrame} from './EditorTransformFrame';
export function moveFrame(frame:TransformFrame,dx:number,dy:number,width:number,height:number):TransformFrame {
 return {...frame,x:Math.round(Math.max(0,Math.min(width-frame.width,frame.x+dx))),y:Math.round(Math.max(0,Math.min(height-frame.height,frame.y+dy)))};
}
/** Opposite corner/edge stays fixed; free side handles change only their own axis. */
export function resizeFrame(frame:TransformFrame,handle:string,dx:number,dy:number,width:number,height:number,ratio:number|null,bounded=true):TransformFrame {
 const horizontal=handle.includes('l')||handle.includes('r'),vertical=handle.includes('t')||handle.includes('b');
 let w=horizontal?Math.max(1,frame.width+(handle.includes('l')?-dx:dx)):frame.width;
 let h=vertical?Math.max(1,frame.height+(handle.includes('t')?-dy:dy)):frame.height;
 const anchorX=handle.includes('l')?frame.x+frame.width:handle.includes('r')?frame.x:frame.x+frame.width/2;
 const anchorY=handle.includes('t')?frame.y+frame.height:handle.includes('b')?frame.y:frame.y+frame.height/2;
 const maxW=bounded?(handle.includes('l')?anchorX:handle.includes('r')?width-anchorX:2*Math.min(anchorX,width-anchorX)):32768;
 const maxH=bounded?(handle.includes('t')?anchorY:handle.includes('b')?height-anchorY:2*Math.min(anchorY,height-anchorY)):32768;
 if(ratio){if(horizontal&&(!vertical||Math.abs(dx)>=Math.abs(dy)*ratio))h=w/ratio;else w=h*ratio;const shrink=Math.min(1,maxW/w,maxH/h);w*=shrink;h*=shrink;}
 else{w=Math.min(w,maxW);h=Math.min(h,maxH);}
 w=Math.max(1,Math.round(w));h=Math.max(1,Math.round(h));
 return {x:Math.round(handle.includes('l')?anchorX-w:handle.includes('r')?anchorX:anchorX-w/2),y:Math.round(handle.includes('t')?anchorY-h:handle.includes('b')?anchorY:anchorY-h/2),width:w,height:h};
}
