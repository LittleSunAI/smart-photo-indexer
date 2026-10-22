import {useEffect,useRef,type RefObject} from 'react';

// Dialogs can open above another dialog (for example an error above details).
// Only the latest mounted owner handles keys and redirects focus.
const owners:Array<{root:RefObject<HTMLElement|null>}>=[];

/** Capture modal keys before bubbling shortcuts; window-level hints get Escape first. */
export function useDialogFocus(root:RefObject<HTMLElement|null>,onClose:()=>void,busy=false){
 const close=useRef(onClose),locked=useRef(busy);close.current=onClose;locked.current=busy;
 useEffect(()=>{
  const owner={root};owners.push(owner);
  const isOwner=()=>{
   if(owners.at(-1)!==owner)return false;
   // Native HTML dialogs already own their top-layer keyboard/focus behavior,
   // including dialogs which do not need this hook. Yield to that modal too.
   const active=document.activeElement;
   const modal=active instanceof Element?active.closest('dialog:modal'):null;
   return !modal||modal===root.current||Boolean(root.current&&modal.contains(root.current));
  };
  const previous=document.activeElement as HTMLElement|null;
  const items=()=>Array.from(root.current?.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),textarea:not(:disabled),select:not(:disabled),[tabindex="0"]')??[]).filter(n=>n.getClientRects().length);
  items().find(n=>n.matches('input,select,textarea'))?.focus();if(!root.current?.contains(document.activeElement))items()[0]?.focus();
  const key=(e:KeyboardEvent)=>{
   if(!isOwner())return;
   if(e.key==='Escape'){e.preventDefault();e.stopImmediatePropagation();if(!locked.current)close.current();return;}
   if(e.key==='Tab'){
    const list=items(),index=list.indexOf(document.activeElement as HTMLElement);
    if(index<0||e.shiftKey&&index===0||!e.shiftKey&&index===list.length-1){e.preventDefault();((e.shiftKey?list.at(-1):list[0])??root.current)?.focus();}
   }
   if(e.key!=='Tab')e.stopPropagation();
  };
  const focus=(e:FocusEvent)=>{if(isOwner()&&root.current&&!root.current.contains(e.target as Node))(items()[0]??root.current)?.focus();};
  document.addEventListener('keydown',key,true);document.addEventListener('focusin',focus);
  return()=>{const active=isOwner();const index=owners.indexOf(owner);if(index>=0)owners.splice(index,1);document.removeEventListener('keydown',key,true);document.removeEventListener('focusin',focus);if(active&&previous?.isConnected)previous.focus();};
 },[root]);
}

/** Closed retained panels must not suppress workspace shortcuts. */
export function hasVisibleDialog(): boolean {
 return [...document.querySelectorAll<HTMLElement>('dialog[open],[role="dialog"]')].some(dialog =>
  !dialog.closest('[hidden],[inert]') && dialog.getClientRects().length > 0);
}
