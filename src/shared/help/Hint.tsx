import {cloneElement,useCallback,type ReactElement,type Ref} from 'react';
import {useHint} from './useHint';
import type {HintOptions} from './HintController';

/** Adds help to one existing DOM element, preserving its original ref and layout. */
export function Hint({children,...options}:HintOptions&{children:ReactElement}){
 const hintRef=useHint<HTMLElement>(options);
 const original=(children.props as {ref?:Ref<HTMLElement>}).ref;
 const ref=useCallback((node:HTMLElement|null)=>{
  hintRef(node);
  if(typeof original==='function')return original(node);
  if(original)original.current=node;
 },[hintRef,original]);
 return cloneElement(children as ReactElement<{ref:Ref<HTMLElement>}>,{ref});
}
