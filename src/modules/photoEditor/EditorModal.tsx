import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useLayoutEffect,useRef,type ReactNode} from 'react';
import {createPortal} from 'react-dom';
import {Icon} from '../../shared/ui/Icon';
import './editor-modal.css';

const dialogFocus=new WeakMap<HTMLDialogElement,HTMLElement>();
export function EditorModal({title,onClose,children,busy=false,modeless=false,className='',returnFocus}:{title:string;onClose():void;children:ReactNode;busy?:boolean;modeless?:boolean;className?:string;returnFocus?:HTMLElement|null}){
  useCurrentLocale();
 const ref=useRef<HTMLDialogElement>(null);
 useLayoutEffect(()=>{
  const root=ref.current;
  if(!root)return;
  const parent=Array.from(document.querySelectorAll<HTMLDialogElement>('.editor-modal[open]')).filter(dialog=>dialog!==root).at(-1);
  const active=document.activeElement instanceof HTMLElement?document.activeElement:null;
  const previous=returnFocus??(active&&active!==document.body?active:parent?dialogFocus.get(parent)??parent:null);
  if(modeless)root.show();else root.showModal();
  return()=>{root.close();if(previous?.isConnected&&!previous.hasAttribute('disabled'))previous.focus();else if(parent?.open)parent.querySelector<HTMLElement>('button:not(:disabled),input:not(:disabled),[tabindex="0"]')?.focus();};
 },[modeless,returnFocus]);
 return createPortal(<dialog ref={ref} className={`editor-modal${modeless?' editor-modal--modeless':''} ${className}`} role="dialog" aria-modal={!modeless} aria-label={title}
  onFocusCapture={event=>{if(ref.current?.contains(event.target))dialogFocus.set(ref.current,event.target);}}
  onCancel={event=>{event.preventDefault();if(!busy)onClose();}}
  onKeyDown={event=>{event.stopPropagation();if(modeless&&event.key==='Escape'){event.preventDefault();if(!busy)onClose();}}}>
  <header><strong>{title}</strong><Hint message={{id:"common.close-dialog"}}><button type="button" aria-label={localeText("editor", "ui.EditorModal.48a36db393595ad9")} disabled={busy} onClick={onClose}><Icon name="close"/></button></Hint></header>
  {children}
 </dialog>,document.body);
}
