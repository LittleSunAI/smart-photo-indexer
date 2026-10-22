import {useEffect,useLayoutEffect,useRef,type PointerEvent as ReactPointerEvent, type RefObject} from 'react';
import {marqueeIds,type MarqueeGeometry} from './MarqueeGeometry';

interface Gesture {pointer:number;x:number;y:number;startX:number;startY:number;active:boolean;time:number;ids:string[]|null}
export function useMarqueeSelection({root,items,geometry,context,view,blockedFocus,suppressedClick,onSelect}:{root:RefObject<HTMLDivElement|null>;items:{id:string}[];geometry:MarqueeGeometry;context?:string;view:string;blockedFocus:{current:boolean};suppressedClick:{current:boolean};onSelect(ids:string[]):void}){
 const latest=useRef({items,geometry,onSelect}),gesture=useRef<Gesture|null>(null),frame=useRef(0),overlay=useRef<HTMLDivElement|null>(null),cleanup=useRef<(()=>void)|null>(null);
 latest.current={items,geometry,onSelect};
 const stop=()=>{cleanup.current?.();cleanup.current=null;cancelAnimationFrame(frame.current);frame.current=0;gesture.current=null;blockedFocus.current=false;overlay.current?.remove();overlay.current=null;};
 const tick=(time:number)=>{
  frame.current=0;const current=gesture.current,node=root.current;if(!current||!node||!current.active)return;
  const bounds=node.getBoundingClientRect(),elapsed=Math.min(32,Math.max(0,time-current.time));current.time=time;
  const edge=36,upper=bounds.top+Math.min(latest.current.geometry.headerHeight,36),lower=bounds.top+node.clientHeight;
  const speed=current.y<upper+edge?-Math.min(1,(upper+edge-current.y)/edge):current.y>lower-edge?Math.min(1,(current.y-lower+edge)/edge):0;
  const oldScroll=node.scrollTop;if(speed)node.scrollTop+=speed*900*elapsed/1000;
  const x=Math.max(bounds.left,Math.min(bounds.left+node.clientWidth,current.x))-bounds.left+node.scrollLeft,y=Math.max(upper,Math.min(lower,current.y))-bounds.top+node.scrollTop;
  const rect={left:Math.min(current.startX,x),top:Math.min(current.startY,y),right:Math.max(current.startX,x),bottom:Math.max(current.startY,y)};
  const ids=marqueeIds(latest.current.items,latest.current.geometry,rect);
  if(!current.ids||ids.length!==current.ids.length||ids.some((id,i)=>id!==current.ids![i])){current.ids=ids;latest.current.onSelect(ids);}
  const left=Math.max(bounds.left,rect.left+bounds.left-node.scrollLeft),top=Math.max(upper,rect.top+bounds.top-node.scrollTop),right=Math.min(bounds.left+node.clientWidth,rect.right+bounds.left-node.scrollLeft),bottom=Math.min(lower,rect.bottom+bounds.top-node.scrollTop);
  if(overlay.current)Object.assign(overlay.current.style,{left:`${left}px`,top:`${top}px`,width:`${Math.max(0,right-left)}px`,height:`${Math.max(0,bottom-top)}px`});
  if(speed&&node.scrollTop!==oldScroll)schedule();
 };
 const schedule=()=>{if(!frame.current)frame.current=requestAnimationFrame(tick);};
 useLayoutEffect(()=>{if(gesture.current?.active)schedule();},[items,geometry]);
 useEffect(()=>()=>stop(),[context,view]);
 return (event:ReactPointerEvent<HTMLDivElement>)=>{
  if(event.button!==0||event.shiftKey||event.ctrlKey||event.metaKey||event.altKey||event.target instanceof Element&&event.target.closest('button,input,select,textarea,a,[role=separator],.explorer-table__header'))return;
  stop();suppressedClick.current=false;const node=root.current;if(!node)return;node.focus({preventScroll:true});const bounds=node.getBoundingClientRect();
  gesture.current={pointer:event.pointerId,x:event.clientX,y:event.clientY,startX:event.clientX-bounds.left+node.scrollLeft,startY:event.clientY-bounds.top+node.scrollTop,active:false,time:performance.now(),ids:null};
  const move=(e:PointerEvent)=>{const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;g.x=e.clientX;g.y=e.clientY;if(!g.active&&Math.hypot(e.clientX-event.clientX,e.clientY-event.clientY)>4){g.active=true;blockedFocus.current=true;overlay.current=document.createElement('div');overlay.current.className='explorer-grid__selection-rectangle';node.append(overlay.current);}if(g.active){e.preventDefault();schedule();}};
  const up=(e:PointerEvent)=>{const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;if(g.active){g.x=e.clientX;g.y=e.clientY;tick(performance.now());suppressedClick.current=true;e.preventDefault();}stop();};
  const cancel=()=>stop(),key=(e:KeyboardEvent)=>{if(e.key==='Escape')stop();},scroll=()=>schedule();
  window.addEventListener('pointermove',move,{passive:false});window.addEventListener('pointerup',up);window.addEventListener('pointercancel',cancel);window.addEventListener('blur',cancel);window.addEventListener('keydown',key);node.addEventListener('scroll',scroll,{passive:true});
  cleanup.current=()=>{window.removeEventListener('pointermove',move);window.removeEventListener('pointerup',up);window.removeEventListener('pointercancel',cancel);window.removeEventListener('blur',cancel);window.removeEventListener('keydown',key);node.removeEventListener('scroll',scroll);};
 };
}
