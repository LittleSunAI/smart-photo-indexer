import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useLayoutEffect,useMemo,useRef,useState,type ReactNode} from 'react';

import {useUiPreferences} from '../../shared/useUiPreferences';
import {saveUiPreferences} from '../../shared/UiPreferences';

interface Item {id:string;estimate:number;content:ReactNode}
/** Expanded annotation cards have measured heights; list identity and scroll anchor stay stable. */
export function EditorMeasuredList({items,reveal}:{items:Item[];reveal?:{id:string;sequence:number}|null}){
  useCurrentLocale();
 const prefs=useUiPreferences(),[desired,setDesired]=useState<number|null>(prefs.editorMarkupHeight),[resizeError,setResizeError]=useState(''),sizing=useRef<{pointerId:number;y:number;height:number;original:number|null}|null>(null),desiredRef=useRef(desired);desiredRef.current=desired;
 useEffect(()=>{if(!sizing.current)setDesired(prefs.editorMarkupHeight);},[prefs.editorMarkupHeight]);
 const persist=(height:number|null)=>{try{saveUiPreferences({editorMarkupHeight:height});setResizeError('');}catch{setResizeError(localeText("editor", "ui.EditorMeasuredList.9dddfbf49b0a772e"));}};
 const root=useRef<HTMLDivElement>(null),[heights,setHeights]=useState<Record<string,number>>({}),[view,setView]=useState({top:0,height:180}),[available,setAvailable]=useState(180);
 const offsets=useMemo(()=>{let total=0;const rows=items.map(item=>{const top=total;total+=heights[item.id]??item.estimate;return {...item,top,height:heights[item.id]??item.estimate};});return {rows,total};},[items,heights]);
 useLayoutEffect(()=>{const el=root.current,inspector=el?.closest<HTMLElement>('.editor-inspector-content');if(!el||!inspector)return;let frame=0;const update=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>{const model=inspector.querySelector<HTMLElement>('.editor-models-panel'),instruction=inspector.querySelector<HTMLElement>('.editor-instruction-panel'),markup=el.closest<HTMLElement>('.editor-markup');if(!model||!instruction||!markup)return;const overhead=markup.offsetHeight-el.offsetHeight;const css=getComputedStyle(inspector),gaps=parseFloat(css.rowGap)||0;const next=Math.max(144,Math.floor(inspector.clientHeight-model.offsetHeight-instruction.offsetHeight-overhead-parseFloat(css.paddingTop)-parseFloat(css.paddingBottom)-gaps-9));setAvailable(old=>old===next?old:next);});};const observer=new ResizeObserver(update);observer.observe(inspector);for(const node of inspector.querySelectorAll('.editor-models-panel,.editor-instruction-panel,.editor-markup>summary,.editor-markup>.editor-card,.editor-markup>.editor-inline-field'))observer.observe(node);update();return()=>{cancelAnimationFrame(frame);observer.disconnect();};},[]);
 const height=desired??Math.min(offsets.total,280,available);
 const previous=useRef(offsets);
 useLayoutEffect(()=>{const el=root.current,old=previous.current;if(el&&old!==offsets){const anchor=old.rows.find(row=>row.top+row.height>el.scrollTop),next=anchor?offsets.rows.find(row=>row.id===anchor.id):null;if(anchor&&next&&Math.abs(next.top-anchor.top)>.5)el.scrollTop+=next.top-anchor.top;}previous.current=offsets;},[offsets]);
 useEffect(()=>{const el=root.current;if(!el)return;let frame=0;const update=()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(()=>setView({top:el.scrollTop,height:el.clientHeight}));};const observer=new ResizeObserver(update);observer.observe(el);el.addEventListener('scroll',update,{passive:true});update();return()=>{cancelAnimationFrame(frame);observer.disconnect();el.removeEventListener('scroll',update);};},[]);
 useLayoutEffect(()=>{if(!reveal||!root.current)return;const row=offsets.rows.find(r=>r.id===reveal.id);if(row)root.current.scrollTop=row.top;},[reveal]);
 const measure=(id:string,height:number)=>setHeights(old=>Math.abs((old[id]??0)-height)<.5?old:{...old,[id]:height});
 const first=offsets.rows.findIndex(row=>row.top+row.height>view.top),start=Math.max(0,first-2),limit=view.top+view.height;
 let end=start;while(end<offsets.rows.length&&offsets.rows[end].top<limit)end++;end=Math.min(offsets.rows.length,end+2);
 const finish=(pointerId:number,cancel=false)=>{const drag=sizing.current;if(!drag||drag.pointerId!==pointerId)return;sizing.current=null;if(cancel)setDesired(drag.original);else persist(desiredRef.current);};
 return <div className="editor-markup-sizer"><div ref={root} className="editor-measured-list" aria-label={localeText("editor", "ui.EditorMeasuredList.1a99d1284025cd3f")} style={{height}}><div style={{height:offsets.total,position:'relative'}}>{offsets.rows.slice(start,end).map(item=><MeasuredRow key={item.id} id={item.id} top={item.top} onMeasure={measure}>{item.content}</MeasuredRow>)}</div></div><Hint message={{id:"layout.markup-height"}}><div className="editor-markup-resize" role="separator" aria-label={localeText("editor", "ui.EditorMeasuredList.c2a93e8312d2eba6")} aria-orientation="horizontal" aria-valuemin={36} aria-valuemax={1200} aria-valuenow={Math.round(height)} tabIndex={0} title={localeText("editor", "ui.EditorMeasuredList.ca0aa0561b23f89a")} onPointerDown={e=>{if(e.button!==0)return;e.preventDefault();e.stopPropagation();e.currentTarget.setPointerCapture(e.pointerId);sizing.current={pointerId:e.pointerId,y:e.clientY,height,original:desired};}} onPointerMove={e=>{const drag=sizing.current;if(!drag||drag.pointerId!==e.pointerId)return;e.preventDefault();const next=Math.max(36,Math.min(1200,Math.round(drag.height+e.clientY-drag.y)));desiredRef.current=next;setDesired(next);}} onPointerUp={e=>{finish(e.pointerId);if(e.currentTarget.hasPointerCapture(e.pointerId))e.currentTarget.releasePointerCapture(e.pointerId);}} onPointerCancel={e=>finish(e.pointerId,true)} onLostPointerCapture={e=>finish(e.pointerId)} onDoubleClick={()=>{setDesired(null);persist(null);}} onKeyDown={e=>{if(!['ArrowUp','ArrowDown','Home','End'].includes(e.key))return;e.preventDefault();const next=e.key==='Home'?36:e.key==='End'?1200:Math.max(36,Math.min(1200,height+(e.key==='ArrowDown'?16:-16)));setDesired(next);persist(next);}}><span/></div></Hint>{resizeError?<span className="editor-muted" role="status">{resizeError}</span>:null}</div>;

}
function MeasuredRow({id,top,onMeasure,children}:{id:string;top:number;onMeasure(id:string,height:number):void;children:ReactNode}){
 const ref=useRef<HTMLDivElement>(null),latest=useRef(onMeasure);latest.current=onMeasure;
 useLayoutEffect(()=>{const el=ref.current;if(!el)return;const measure=()=>latest.current(id,el.getBoundingClientRect().height);measure();const observer=new ResizeObserver(measure);observer.observe(el);return()=>observer.disconnect();},[id]);
 return <div ref={ref} style={{position:'absolute',left:0,right:0,top}}>{children}</div>;
}
