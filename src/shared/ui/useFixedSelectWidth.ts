import {useLayoutEffect,useRef,useState,type CSSProperties} from 'react';

/** The reference label defines the control width; selection never changes it. */
export function useFixedSelectWidth(reference:string){
 const ref=useRef<HTMLSelectElement>(null),[width,setWidth]=useState(160);
  useLayoutEffect(()=>{let active=true;const measure=()=>{const element=ref.current,context=document.createElement('canvas').getContext('2d');if(!active||!element||!context)return;const font=getComputedStyle(element);context.font=`${font.fontWeight} ${parseFloat(font.fontSize)>0?font.fontSize:'13px'} ${font.fontFamily}`;setWidth(Math.ceil(context.measureText(reference).width)+27);};measure();void document.fonts?.ready.then(measure);const observer=new MutationObserver(measure);if(ref.current)observer.observe(ref.current,{childList:true,subtree:true,characterData:true});return()=>{active=false;observer.disconnect();};},[reference]);
 return {ref,style:{'--fixed-select-width':`${width}px`} as CSSProperties};
}
