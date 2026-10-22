import {useLayoutEffect,useRef,useState} from 'react';

/** Measure normal text variants independently of the currently compact DOM. */
export function useMeasuredToolbar(attribute:string,variants:readonly string[],configuration:string,preserveShrink=false,wrapperClass=''){
 const ref=useRef<HTMLDivElement>(null),[level,setLevel]=useState(0);
 const state=useRef({level:0,viewport:window.innerWidth});
 useLayoutEffect(()=>{
  const element=ref.current;if(!element)return;
  let active=true,frame=0;state.current={level:0,viewport:window.innerWidth};
  const measure=()=>{
   frame=0;if(!active||!element.getClientRects().length||element.clientWidth===0)return;
   const widths=variants.map(value=>{
    const clone=element.cloneNode(true) as HTMLElement;
    clone.setAttribute(attribute,value);clone.setAttribute('aria-hidden','true');clone.inert=true;
    clone.style.cssText+=';position:static;width:max-content;min-width:0;max-width:none;flex:none;';
    const host=document.createElement('div');host.className=wrapperClass;
    host.style.cssText='position:fixed;left:-100000px;top:0;width:max-content;max-width:none;visibility:hidden;pointer-events:none;';
    host.style.font=getComputedStyle(element).font;host.appendChild(clone);document.body.appendChild(host);
    const width=clone.getBoundingClientRect().width;host.remove();return width;
   });
   element.dataset.fitWidths=widths.map(Math.ceil).join(',');
   let next=widths.findIndex(width=>width<=element.clientWidth+.5);if(next<0)next=variants.length-1;
   const previous=state.current;
   if(preserveShrink&&innerWidth<=previous.viewport)next=Math.max(next,previous.level);
   state.current={level:next,viewport:innerWidth};
   setLevel(old=>old===next?old:next);
  };
  const schedule=()=>{if(!frame)frame=requestAnimationFrame(measure);};
  const explicit=()=>{state.current.level=0;schedule();};
  const resize=new ResizeObserver(schedule);resize.observe(element);
  const mutation=new MutationObserver(schedule);mutation.observe(element,{subtree:true,childList:true,characterData:true,attributes:true,attributeFilter:['class','hidden']});
  window.addEventListener('resize',schedule);window.addEventListener('smart-photo-indexer:panel-width-intent',explicit);
  void document.fonts?.ready.then(()=>{if(active)explicit();});measure();
  return()=>{active=false;cancelAnimationFrame(frame);resize.disconnect();mutation.disconnect();window.removeEventListener('resize',schedule);window.removeEventListener('smart-photo-indexer:panel-width-intent',explicit);};
 },[attribute,configuration,preserveShrink,wrapperClass,variants.join('|')]);
 return {ref,level,value:variants[level]};
}
