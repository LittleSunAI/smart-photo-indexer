import type {ReactNode} from 'react';
import type {HelpMessage} from './HelpMessage';
import {loadUiPreferences,UI_PREFERENCES_EVENT,UI_PREFERENCES_KEY} from '../UiPreferences';

export interface HintOptions {
 message?:HelpMessage;
 content?:ReactNode;
 reason?:string;
 supplement?:HelpMessage;
 details?:ReactNode;
 popupOpen?:boolean;
 explicit?:boolean;
 /** This explicit help opens only by activation, never by pointer or focus delay. */
 activationOnly?:boolean;
 /** Keep the hints preference control discoverable while other automatic help is off. */
 availableWhenDisabled?:boolean;
 /** Resolve a control inside an existing display:contents adapter. */
 target?:(root:HTMLElement)=>HTMLElement|null;
}
export interface VisibleHint {anchor:HTMLElement;options:HintOptions;pinned:boolean;portal:HTMLElement}
const delay=700,grace=160;

/** Document-local presentation state only. No application actions or IPC. */
class HintController {
 private targets=new Map<HTMLElement,()=>HintOptions>();
 private generatedLabels=new WeakMap<HTMLElement,string>();
 private subscribers=new Set<()=>void>();
 private current:VisibleHint|null=null;
 private timer:ReturnType<typeof setTimeout>|null=null;
 private hovered:HTMLElement|null=null;
 private blocked:HTMLElement|null=null;
 private keyboardIntent=false;
 private automaticEnabled=true;
 private pending:HTMLElement|null=null;
 note:HTMLElement|null=null;
 readonly tooltipId='smart-photo-hint';
 subscribe=(listener:()=>void)=>{this.subscribers.add(listener);return()=>{this.subscribers.delete(listener);};};
 snapshot=()=>this.current;
 private emit(){for(const listener of this.subscribers)listener();}
 private clearTimer(){if(this.timer!==null)clearTimeout(this.timer);this.timer=null;this.pending=null;}
 private describe(anchor:HTMLElement,add:boolean){
  const ids=(anchor.getAttribute('aria-describedby')??'').split(/\s+/).filter(id=>id&&id!==this.tooltipId);
  if(add)ids.push(this.tooltipId);
  if(ids.length)anchor.setAttribute('aria-describedby',ids.join(' '));else anchor.removeAttribute('aria-describedby');
  if(this.targets.get(anchor)?.().explicit){anchor.setAttribute('aria-expanded',String(add));if(add)anchor.setAttribute('aria-controls',this.tooltipId);else anchor.removeAttribute('aria-controls');}
 }
 close=()=>{this.clearTimer();if(!this.current)return;this.describe(this.current.anchor,false);this.current=null;this.emit();};
 private preserveAccessibleName(anchor:HTMLElement){
  const title=anchor.getAttribute('title');if(!title||anchor.hasAttribute('aria-labelledby'))return;
  const prior=this.generatedLabels.get(anchor);
  if(prior?anchor.getAttribute('aria-label')===prior:!anchor.hasAttribute('aria-label')&&!anchor.textContent?.trim()){
   anchor.setAttribute('aria-label',title);this.generatedLabels.set(anchor,title);
  }
 }
 register(anchor:HTMLElement,read:()=>HintOptions){
  this.targets.set(anchor,read);anchor.dataset.hintAnchor='true';if(read().message)anchor.dataset.hintId=read().message!.id;
  this.preserveAccessibleName(anchor);
  const titled=[anchor,...anchor.querySelectorAll<HTMLElement>('.compact-select__value[title]')].filter(node=>node.hasAttribute('title'));
  const titles=titled.map(node=>[node,node.getAttribute('title')!] as const);for(const [node]of titles)node.removeAttribute('title');
  return()=>{
   if(this.current?.anchor===anchor||this.pending===anchor)this.close();
   this.targets.delete(anchor);delete anchor.dataset.hintAnchor;delete anchor.dataset.hintId;
   for(const [node,title]of titles)if(node.isConnected&&!node.hasAttribute('title'))node.setAttribute('title',title);
   const label=this.generatedLabels.get(anchor);if(label&&anchor.getAttribute('aria-label')===label)anchor.removeAttribute('aria-label');this.generatedLabels.delete(anchor);
   if(this.hovered===anchor)this.hovered=null;if(this.blocked===anchor)this.blocked=null;
  };
 }
 refresh(anchor:HTMLElement){
  const id=this.targets.get(anchor)?.().message?.id;if(id)anchor.dataset.hintId=id;else delete anchor.dataset.hintId;
  this.preserveAccessibleName(anchor);
  for(const node of [anchor,...anchor.querySelectorAll<HTMLElement>('.compact-select__value[title]')])node.removeAttribute('title');
  if(this.pending===anchor&&!this.eligible(anchor))this.clearTimer();
  if(this.current?.anchor!==anchor)return;
  if(!this.eligible(anchor,this.current.pinned)){this.close();return;}
  this.current={...this.current,options:this.targets.get(anchor)!()};this.emit();
 }
 private find(node:EventTarget|null):HTMLElement|null{
  let element=node instanceof Element?node:node instanceof Node?node.parentElement:null;
  while(element){if(element instanceof HTMLElement&&this.targets.has(element))return element;element=element.parentElement;}
  return null;
 }
 private modal(){return [...document.querySelectorAll<HTMLDialogElement>('dialog[open]')].filter(dialog=>dialog.matches(':modal')).at(-1)??null;}
 private eligible(anchor:HTMLElement,pinned=false){
  const options=this.targets.get(anchor)?.();if(!options||!anchor.isConnected||!anchor.getClientRects().length)return false;
  if(!pinned&&options.activationOnly)return false;
  if(!pinned&&!this.automaticEnabled&&!options.availableWhenDisabled)return false;
  const modal=this.modal();if(modal&&!modal.contains(anchor))return false;
  if(!pinned&&document.querySelector('.help-tour-blocker'))return false;
  if(!options.explicit&&anchor.closest('[role="menu"],[role="listbox"]'))return false;
  if(options.popupOpen||(!options.explicit&&anchor.matches('[aria-haspopup][aria-expanded="true"]')))return false;
  const field=anchor.matches('input,textarea,select,[contenteditable="true"]')?anchor:anchor.querySelector('input,textarea,select,[contenteditable="true"]');
  if(!pinned&&field&&document.activeElement===field)return false;
  return true;
 }
 private show(anchor:HTMLElement,pinned:boolean){
  if(!this.eligible(anchor,pinned))return;
  this.close();this.current={anchor,options:this.targets.get(anchor)!(),pinned,portal:this.modal()??document.body};this.describe(anchor,true);this.emit();
 }
 private schedule(anchor:HTMLElement,keyboard=false){
  if(this.current?.pinned||this.blocked===anchor||!this.eligible(anchor))return;
  this.clearTimer();this.pending=anchor;
  this.timer=setTimeout(()=>{
   this.timer=null;this.pending=null;
   if(keyboard?document.activeElement===anchor:this.hovered===anchor)this.show(anchor,false);
  },delay);
 }
 private over=(event:PointerEvent)=>{
  if(this.note?.contains(event.target as Node)){this.clearTimer();return;}
  const anchor=this.find(event.target);if(anchor===this.hovered)return;
  this.hovered=anchor;if(!this.current?.pinned)this.close();
  if(anchor&&event.buttons===0)this.schedule(anchor);
 };
 private out=(event:PointerEvent)=>{
  const next=this.find(event.relatedTarget),previous=this.find(event.target);
  if(previous&&previous!==next&&this.blocked===previous)this.blocked=null;
  if(previous===next&&!this.note?.contains(event.target as Node))return;
  if(next===this.current?.anchor||this.note?.contains(event.relatedTarget as Node)){this.clearTimer();return;}
  this.hovered=null;this.clearTimer();
  if(this.current&&!this.current.pinned)this.timer=setTimeout(()=>this.close(),grace);
 };
 private pointer=(event:PointerEvent)=>{
  this.keyboardIntent=false;this.clearTimer();this.blocked=this.find(event.target);
  if(this.note?.contains(event.target as Node))return;
  if(this.current?.pinned&&this.current.anchor.contains(event.target as Node))return;
  this.close();
 };
 private click=(event:MouseEvent)=>{
  const anchor=this.find(event.target);if(!anchor||!this.targets.get(anchor)?.().explicit)return;
  event.preventDefault();event.stopPropagation();
  if(this.current?.anchor===anchor&&this.current.pinned)this.close();else this.show(anchor,true);
 };
 private focus=(event:FocusEvent)=>{
  if(this.note?.contains(event.target as Node))return;
  const anchor=this.find(event.target);
  if(this.current&&(this.current.anchor!==anchor||!this.eligible(this.current.anchor,this.current.pinned)))this.close();
  const keyboard=this.keyboardIntent;this.keyboardIntent=false;
  if(keyboard&&anchor){this.blocked=null;this.schedule(anchor,true);}
 };
 private blurFocus=(event:FocusEvent)=>{
  if(this.current?.pinned)return;
  if(this.find(event.relatedTarget)!==this.find(event.target))this.close();
 };
 private key=(event:KeyboardEvent)=>{
  if(event.key==='Tab'){this.keyboardIntent=true;this.blocked=null;this.clearTimer();return;}
  if(event.key==='Escape'){
   const pinned=this.current?.pinned;
   this.blocked=this.current?.anchor??this.pending;this.close();
   if(pinned){event.preventDefault();event.stopImmediatePropagation();}return;
  }
  if(event.key==='Enter'||event.key===' '){
   this.keyboardIntent=false;this.clearTimer();const anchor=this.find(event.target);this.blocked=anchor;
   if(anchor&&this.targets.get(anchor)?.().explicit){event.stopPropagation();return;}
  }
  if(!this.current?.pinned)this.close();
 };
 private interaction=(event:Event)=>{
  if(this.note?.contains(event.target as Node))return;
  this.blocked=this.pending??this.current?.anchor??this.hovered;this.clearTimer();if(!this.current?.pinned)this.close();
 };
 private windowBlur=()=>{this.blocked=this.hovered;this.keyboardIntent=false;this.close();};
 private refreshPreferences=()=>{
  const enabled=loadUiPreferences().hoverHintsEnabled;
  if(enabled===this.automaticEnabled)return;
  this.automaticEnabled=enabled;
  if(!enabled){if(this.pending&&!this.eligible(this.pending))this.clearTimer();this.keyboardIntent=false;if(this.current&&!this.current.pinned&&!this.eligible(this.current.anchor))this.close();}
 };
 private preferenceStorage=(event:StorageEvent)=>{if(event.key===null||event.key===UI_PREFERENCES_KEY)this.refreshPreferences();};
 mount(){
  window.addEventListener(UI_PREFERENCES_EVENT,this.refreshPreferences);
  window.addEventListener('storage',this.preferenceStorage);
  this.refreshPreferences();
  const events=[['pointerover',this.over],['pointerout',this.out],['pointerdown',this.pointer],['click',this.click],['focusin',this.focus],['focusout',this.blurFocus],['keydown',this.key],['input',this.interaction],['wheel',this.interaction],['gotpointercapture',this.interaction]] as const;
  for(const [name,listener]of events)document.addEventListener(name,listener as EventListener,true);
  window.addEventListener('blur',this.windowBlur);
  window.addEventListener('smart-photo-indexer:open-help',this.close);
  return()=>{for(const [name,listener]of events)document.removeEventListener(name,listener as EventListener,true);window.removeEventListener('blur',this.windowBlur);window.removeEventListener('smart-photo-indexer:open-help',this.close);window.removeEventListener(UI_PREFERENCES_EVENT,this.refreshPreferences);window.removeEventListener('storage',this.preferenceStorage);this.keyboardIntent=false;this.close();};
 }
}
export const hints=new HintController();
