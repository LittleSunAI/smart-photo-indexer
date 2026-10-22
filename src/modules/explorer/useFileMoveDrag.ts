import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useLayoutEffect,useRef,type PointerEvent as ReactPointerEvent,type RefObject} from 'react';
import {useAppSelector} from '../../app/state/AppStateContext';
import {isQueryView,quickSearchView} from '../../app/state/QuickSearch';
import {requestFileCommand,type FileActionSource} from '../filesystem/FileActions';
import type {ExplorerItem} from './types';
import '../filesystem/file-actions.css';

interface Gesture {pointer:number;startX:number;startY:number;x:number;y:number;active:boolean;source:FileActionSource;count:number;target:HTMLElement|null;folder:string|undefined;time:number;root:HTMLDivElement}
export function useFileMoveDrag({root,items,selectedItemIds,context,view,suppressedClick,blockedFocus}:{root:RefObject<HTMLDivElement|null>;items:ExplorerItem[];selectedItemIds:string[];context?:string;view:string;suppressedClick:{current:boolean};blockedFocus:{current:boolean}}){
  useCurrentLocale();
 const query=useAppSelector(state=>isQueryView(state)?quickSearchView(state).search.selectionSnapshot:undefined);
 const latest=useRef({items,selectedItemIds,query}),gesture=useRef<Gesture|null>(null),cleanup=useRef<(()=>void)|null>(null),frame=useRef(0),badge=useRef<HTMLDivElement|null>(null);
 latest.current={items,selectedItemIds,query};
 const stop=()=>{
  const g=gesture.current;gesture.current=null;cleanup.current?.();cleanup.current=null;cancelAnimationFrame(frame.current);frame.current=0;
  g?.target?.classList.remove('file-move-target');badge.current?.remove();badge.current=null;blockedFocus.current=false;
  if(g?.root.hasPointerCapture(g.pointer))g.root.releasePointerCapture(g.pointer);
 };
 const cancel=()=>{if(gesture.current?.active)suppressedClick.current=true;stop();};
 const targetAt=(g:Gesture)=>{
  const node=document.elementFromPoint(g.x,g.y)?.closest<HTMLElement>('[data-explorer-item-id]');
  const folder=node&&g.root.contains(node)?latest.current.items.find(item=>item.id===node.dataset.explorerItemId&&item.kind==='folder'):undefined;
  if(g.target!==node||!folder){g.target?.classList.remove('file-move-target');g.target=folder?node??null:null;g.target?.classList.add('file-move-target');}
  g.folder=folder?.path;
 };
 const tick=(time:number)=>{
  frame.current=0;const g=gesture.current;if(!g?.active)return;
  const box=g.root.getBoundingClientRect(),edge=36,elapsed=Math.min(32,Math.max(0,time-g.time));g.time=time;
  const speed=g.x<box.left||g.x>box.right?0:g.y<box.top+edge?-Math.min(1,(box.top+edge-g.y)/edge):g.y>box.bottom-edge?Math.min(1,(g.y-box.bottom+edge)/edge):0;
  const old=g.root.scrollTop;if(speed)g.root.scrollTop+=speed*900*elapsed/1000;
  targetAt(g);
  if(badge.current){badge.current.textContent=localeText("main", "ui.useFileMoveDrag.837468f1bc70dc1a", {"value1": (g.count), "value2": (g.folder?' → '+g.folder.split(/[\\/]/).at(-1):'')});Object.assign(badge.current.style,{left:`${Math.max(8,Math.min(innerWidth-badge.current.offsetWidth-8,g.x+16))}px`,top:`${Math.max(8,Math.min(innerHeight-42,g.y+16))}px`});badge.current.dataset.valid=String(Boolean(g.folder));}
  if(speed&&old!==g.root.scrollTop)schedule();
 };
 const schedule=()=>{if(!frame.current)frame.current=requestAnimationFrame(tick);};
 useLayoutEffect(()=>{if(gesture.current?.active)schedule();},[items]);
 useLayoutEffect(()=>()=>cancel(),[context,view]);
 return (event:ReactPointerEvent<HTMLDivElement>):boolean=>{
  if(event.button!==0||!event.altKey)return false;
  if(event.target instanceof Element&&event.target.closest('button,input,textarea,select,a,[role=separator],.explorer-table__header'))return true;
  stop();suppressedClick.current=false;
  const node=root.current,card=event.target instanceof Element?event.target.closest<HTMLElement>('[data-explorer-item-id]'):null;
  const image=card?items.find(item=>item.id===card.dataset.explorerItemId&&item.kind==='image'):undefined;
  if(!node||!image)return true;
  const selected=selectedItemIds.includes(image.id),snapshot=selected?latest.current.query:undefined;
  const paths=selected?items.filter(item=>item.kind==='image'&&selectedItemIds.includes(item.id)).map(item=>item.path):[image.path];
  const source:FileActionSource=snapshot?.input.queryId?{type:'query',queryId:snapshot.input.queryId,excludedPaths:[...snapshot.excludedPaths]}:{type:'explicit',paths:[...new Set(paths)]};
  const count=snapshot?Math.max(0,snapshot.totalCount-snapshot.excludedPaths.length):paths.length;
  node.focus({preventScroll:true});gesture.current={pointer:event.pointerId,startX:event.clientX,startY:event.clientY,x:event.clientX,y:event.clientY,active:false,source,count,target:null,folder:undefined,time:performance.now(),root:node};
  const move=(e:PointerEvent)=>{
   const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;g.x=e.clientX;g.y=e.clientY;
   if(!g.active&&Math.hypot(g.x-g.startX,g.y-g.startY)>4){g.active=true;blockedFocus.current=true;suppressedClick.current=true;node.setPointerCapture(g.pointer);badge.current=document.createElement('div');badge.current.className='file-move-badge';badge.current.setAttribute('role','status');document.body.append(badge.current);}
   if(g.active){e.preventDefault();schedule();}
  };
  const up=(e:PointerEvent)=>{
   const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;
   if(g.active){g.x=e.clientX;g.y=e.clientY;targetAt(g);e.preventDefault();suppressedClick.current=true;}
   const target=g.active?g.folder:undefined,frozen=g.source;stop();
   if(target)requestFileCommand('moveTo',frozen,target);
  };
  const key=(e:KeyboardEvent)=>{if(e.key==='Escape'&&gesture.current){e.preventDefault();e.stopImmediatePropagation();cancel();}};
  window.addEventListener('pointermove',move,{passive:false});window.addEventListener('pointerup',up);window.addEventListener('pointercancel',cancel);window.addEventListener('blur',cancel);window.addEventListener('keydown',key,true);window.addEventListener('contextmenu',cancel,true);node.addEventListener('lostpointercapture',cancel);node.addEventListener('scroll',schedule,{passive:true});
  cleanup.current=()=>{window.removeEventListener('pointermove',move);window.removeEventListener('pointerup',up);window.removeEventListener('pointercancel',cancel);window.removeEventListener('blur',cancel);window.removeEventListener('keydown',key,true);window.removeEventListener('contextmenu',cancel,true);node.removeEventListener('lostpointercapture',cancel);node.removeEventListener('scroll',schedule);};
  return true;
 };
}
