import {useCallback,useLayoutEffect,useRef,useState,type ReactNode} from 'react';

function MeasuredCard({id,top,measure,children}:{id:string;top:number;measure(id:string,height:number):void;children:ReactNode}){
 const ref=useRef<HTMLDivElement>(null);
 useLayoutEffect(()=>{const el=ref.current;if(!el)return;const update=()=>measure(id,Math.ceil(el.getBoundingClientRect().height));update();const observer=new ResizeObserver(update);observer.observe(el);return()=>observer.disconnect();},[id,measure]);
 return <div ref={ref} data-card-id={id} className="measured-card-list__item" style={{position:'absolute',top,left:0,right:0,paddingBottom:6,boxSizing:'border-box'}}>{children}</div>;
}

/** A variable-height list for independent cards; markup and job contents stay outside this engine. */
export function MeasuredCardList<T>({items,itemKey,render,label,estimate=72,maxHeight=440,fillAvailable=false}:{items:T[];itemKey(item:T):string;render(item:T):ReactNode;label:string;estimate?:number;maxHeight?:number;fillAvailable?:boolean}){
 const root=useRef<HTMLDivElement>(null),sizes=useRef(new Map<string,number>()),positions=useRef<Array<{id:string;top:number;height:number}>>([]);
 const [generation,setGeneration]=useState(0),[viewport,setViewport]=useState({top:0,height:maxHeight});
 const measure=useCallback((id:string,height:number)=>{
  if(height<1||sizes.current.get(id)===height)return;
  sizes.current.set(id,height);setGeneration(n=>n+1);
 },[]);
 let total=0;const rows=items.map(item=>{const id=itemKey(item),height=sizes.current.get(id)??estimate,row={id,item,top:total,height};total+=height;return row;});
 // Preserve the visible card across insertion, removal and height changes.
 // Capture before the DOM commit; browser scroll anchoring must not also adjust it.
 const scrollTop=root.current?.scrollTop??0,previous=positions.current;
 const anchorIndex=scrollTop>0?previous.findIndex(row=>row.top+row.height>scrollTop):-1;
 const nextById=new Map(rows.map(row=>[row.id,row]));
 const anchor=anchorIndex<0?undefined:
  previous.slice(anchorIndex).find(row=>nextById.has(row.id))??
  previous.slice(0,anchorIndex).reverse().find(row=>nextById.has(row.id));
 useLayoutEffect(()=>{
  const el=root.current;
  if(el&&anchor){const next=nextById.get(anchor.id);if(next)el.scrollTop=next.top+(scrollTop-anchor.top);}
  positions.current=rows;
  if(el)setViewport(v=>v.top===el.scrollTop&&v.height===el.clientHeight?v:{top:el.scrollTop,height:el.clientHeight});
 });
 useLayoutEffect(()=>{const el=root.current;if(!el)return;const update=()=>setViewport(v=>({...v,top:el.scrollTop,height:el.clientHeight}));const observer=new ResizeObserver(update);observer.observe(el);update();return()=>observer.disconnect();},[items.length]);
 const ids=items.map(itemKey).join('|');useLayoutEffect(()=>{const keep=new Set(items.map(itemKey));for(const id of sizes.current.keys())if(!keep.has(id))sizes.current.delete(id);},[ids]);
 void generation;
 if(!items.length)return null;
 const visible=rows.filter(row=>row.top+row.height>=viewport.top-300&&row.top<=viewport.top+viewport.height+300);
 return <div ref={root} className="measured-card-list" role="region" aria-label={label} style={{height:fillAvailable?'100%':Math.min(maxHeight,total),maxHeight:fillAvailable?'none':'55vh',flex:fillAvailable?'1 1 0':undefined,minHeight:0,overflow:'auto',overflowAnchor:'none',position:'relative'}} onScroll={e=>{const top=e.currentTarget.scrollTop;setViewport(v=>({...v,top}));}}>
  <div style={{height:total,position:'relative'}}>{visible.map(row=><MeasuredCard key={row.id} id={row.id} top={row.top} measure={measure}>{render(row.item)}</MeasuredCard>)}</div>
 </div>;
}
