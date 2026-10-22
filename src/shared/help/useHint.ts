import {useCallback,useLayoutEffect,useRef,useState} from 'react';
import {hints,type HintOptions} from './HintController';

/** Attaches to an existing node; does not create a layout wrapper or event handlers per control. */
export function useHint<T extends HTMLElement>(options:HintOptions){
 const latest=useRef(options);latest.current=options;
 const [root,setRoot]=useState<T|null>(null);
 const registered=useRef<{anchor:HTMLElement;remove:()=>void}|null>(null);
 useLayoutEffect(()=>{
  const anchor=root?(latest.current.target?latest.current.target(root):root):null;
  if(registered.current?.anchor!==anchor){registered.current?.remove();registered.current=anchor?{anchor,remove:hints.register(anchor,()=>latest.current)}:null;}
  if(anchor)hints.refresh(anchor);
 });
 useLayoutEffect(()=>()=>{registered.current?.remove();registered.current=null;},[]);
 return useCallback((node:T|null)=>setRoot(node),[]);
}
