import {lazy,Suspense,useCallback,useEffect,useState} from 'react';
const Center=lazy(()=>import('./HelpCenter'));
/** Each working window owns its local tour; opening help never creates another window. */
export function HelpHost(){
 const [open,setOpen]=useState(false);
 const close=useCallback(()=>{setOpen(false);window.dispatchEvent(new Event('smart-photo-indexer:help-ended'));},[]);
 useEffect(()=>{
  const show=()=>{if(!document.querySelector('dialog[open],[role="dialog"][aria-modal="true"]:not([data-help-overlay="settings"])'))setOpen(true);};
  window.addEventListener('smart-photo-indexer:open-help',show);return()=>window.removeEventListener('smart-photo-indexer:open-help',show);
 },[]);
 return open?<Suspense fallback={null}><Center onClose={close}/></Suspense>:null;
}
