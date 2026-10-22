import{useEffect,useLayoutEffect,useRef,useState,type PointerEvent as ReactPointerEvent}from'react';
import{convertFileSrc}from'@tauri-apps/api/core';
import{text,useCurrentLocale}from'../../shared/i18n/Runtime';
import{Icon}from'../../shared/ui/Icon';
import type{EditorAsset}from'./EditorContracts';
import type{WorkingOrientation}from'./EditorOrientation';
type Size={width:number;height:number};
export interface PreviewBox extends Size{x:number;y:number}
type ImageView={scale:number;panX:number;panY:number;mode:'fit'|'manual'};
export interface PreviewMemory {box:PreviewBox;view:ImageView;decoded:Size;source:string;orientation:string}
type Edge='n'|'s'|'e'|'w'|'ne'|'nw'|'se'|'sw';
const clamp=(n:number,a:number,b:number)=>Math.max(a,Math.min(b,n));
export function previewLimits(size:Size){const gap=Math.max(0,Math.min(8,size.width/8,size.height/8)),width=Math.max(1,size.width-2*gap),height=Math.max(1,size.height-2*gap);return{gap,width,height,minWidth:Math.min(360,width),minHeight:Math.min(260,height)};}
export function clampPreviewBox(box:PreviewBox,size:Size):PreviewBox{const l=previewLimits(size),width=clamp(box.width,l.minWidth,l.width),height=clamp(box.height,l.minHeight,l.height);return{width,height,x:clamp(box.x,l.gap,Math.max(l.gap,size.width-l.gap-width)),y:clamp(box.y,l.gap,Math.max(l.gap,size.height-l.gap-height))};}
export function initialPreviewBox(size:Size):PreviewBox{const width=Math.min(800,size.width*.72),height=Math.min(600,size.height*.8);return clampPreviewBox({width,height,x:(size.width-width)/2,y:(size.height-height)/2},size);}
function resizeBox(box:PreviewBox,size:Size,edge:Edge,dx:number,dy:number):PreviewBox{
 const l=previewLimits(size);let left=box.x,right=box.x+box.width,top=box.y,bottom=box.y+box.height;
 if(edge.includes('w'))left=clamp(left+dx,l.gap,right-l.minWidth);
 if(edge.includes('e'))right=clamp(right+dx,left+l.minWidth,size.width-l.gap);
 if(edge.includes('n'))top=clamp(top+dy,l.gap,bottom-l.minHeight);
 if(edge.includes('s'))bottom=clamp(bottom+dy,top+l.minHeight,size.height-l.gap);
 return clampPreviewBox({x:left,y:top,width:right-left,height:bottom-top},size);
}
interface Props{memory?:PreviewMemory;remember?(state:PreviewMemory):void;asset:EditorAsset;orientation:WorkingOrientation;title:string;workspace:Size;busy:boolean;canHide:boolean;hidden:boolean;onClose():void;onTake():Promise<void>;onVisibility():void}
export function HistoryTreePreview(p:Props){
 useCurrentLocale();
 const restored=useRef(p.memory?.source===p.asset.previewPath&&p.memory.orientation===JSON.stringify(p.orientation)?p.memory:null);
 const geometryInitialized=useRef(p.workspace.width>0&&p.workspace.height>0);
 const[box,setBox]=useState(()=>restored.current?clampPreviewBox(restored.current.box,p.workspace):initialPreviewBox(p.workspace)),boxNow=useRef(box);
 const[view,setView]=useState<ImageView>(restored.current?.view??{scale:1,panX:0,panY:0,mode:'fit'}),viewNow=useRef(view);
 const[decoded,setDecoded]=useState<Size|null>(null),decodedNow=useRef(decoded),[failed,setFailed]=useState(false);
 const dialog=useRef<HTMLDivElement>(null),stage=useRef<HTMLDivElement>(null),image=useRef<HTMLImageElement>(null);
 const gesture=useRef<{pointer:number;owner:HTMLElement;kind:'move'|'resize'|'pan';edge?:Edge;startX:number;startY:number;box:PreviewBox;view:ImageView}|null>(null);
 const commitBox=(next:PreviewBox)=>{boxNow.current=next;setBox(next);};
 const commitView=(next:ImageView)=>{viewNow.current=next;setView(next);};
 const fitScale=()=>{const el=stage.current,size=decodedNow.current;if(!el||!size||el.clientWidth<=0||el.clientHeight<=0)return null;const rotated=p.orientation.rotation%2;return Math.min(8,Math.max(1,el.clientWidth-16)/(rotated?size.height:size.width),Math.max(1,el.clientHeight-16)/(rotated?size.width:size.height));};
 const fitImage=()=>{const scale=fitScale();if(scale&&scale>0)commitView({scale,panX:0,panY:0,mode:'fit'});};
 const fitCallback=useRef(fitImage);fitCallback.current=fitImage;
 const finish=()=>{const active=gesture.current;gesture.current=null;if(active?.owner.hasPointerCapture(active.pointer))active.owner.releasePointerCapture(active.pointer);};
 useLayoutEffect(()=>{finish();decodedNow.current=null;setDecoded(null);setFailed(false);if(!restored.current)commitView({scale:1,panX:0,panY:0,mode:'fit'});},[p.asset.previewPath]);
 useEffect(()=>{const blur=()=>finish();window.addEventListener('blur',blur);dialog.current?.querySelector<HTMLButtonElement>('[data-testid="tree-preview-close"]')?.focus();return()=>{window.removeEventListener('blur',blur);finish();};},[]);
 useLayoutEffect(()=>{if(p.workspace.width<=0||p.workspace.height<=0)return;finish();commitBox(geometryInitialized.current?clampPreviewBox(boxNow.current,p.workspace):initialPreviewBox(p.workspace));geometryInitialized.current=true;},[p.workspace.width,p.workspace.height]);
 useLayoutEffect(()=>{const el=stage.current;if(!el)return;const update=()=>{if(viewNow.current.mode==='fit')fitCallback.current();};const observer=new ResizeObserver(update);observer.observe(el);update();return()=>observer.disconnect();},[]);
 useLayoutEffect(()=>{if(decodedNow.current)fitImage();},[p.orientation.rotation,p.orientation.mirrored]);
 const zoomAt=(factor:number,x:number,y:number)=>{const current=viewNow.current,fit=fitScale();if(!fit)return;const scale=clamp(current.scale*factor,Math.min(.1,fit),8),ratio=scale/current.scale;commitView({scale,panX:x-(x-current.panX)*ratio,panY:y-(y-current.panY)*ratio,mode:'manual'});};
 const wheelHandler=useRef((e:WheelEvent)=>{});wheelHandler.current=e=>{e.preventDefault();e.stopPropagation();if((e.target as Element).closest('button,input'))return;const el=stage.current;if(!el)return;const r=el.getBoundingClientRect(),pixels=e.deltaY*(e.deltaMode===1?16:e.deltaMode===2?el.clientHeight:1);zoomAt(Math.exp(-pixels*.0015),e.clientX-r.left-r.width/2,e.clientY-r.top-r.height/2);};
 useEffect(()=>{const el=stage.current;if(!el)return;const wheel=(e:WheelEvent)=>wheelHandler.current(e);el.addEventListener('wheel',wheel,{passive:false});return()=>el.removeEventListener('wheel',wheel);},[]);
 const start=(e:ReactPointerEvent<HTMLElement>,kind:'move'|'resize'|'pan',edge?:Edge)=>{if(gesture.current||(kind==='pan'?e.button!==1:e.button!==0))return;e.preventDefault();e.stopPropagation();gesture.current={pointer:e.pointerId,owner:e.currentTarget,kind,edge,startX:e.clientX,startY:e.clientY,box:boxNow.current,view:viewNow.current};e.currentTarget.setPointerCapture(e.pointerId);};
 const move=(e:ReactPointerEvent<HTMLElement>)=>{const g=gesture.current;if(!g||g.pointer!==e.pointerId)return;e.preventDefault();const dx=e.clientX-g.startX,dy=e.clientY-g.startY;if(g.kind==='pan'){if(dx||dy)commitView({...g.view,mode:'manual',panX:g.view.panX+dx,panY:g.view.panY+dy});}else commitBox(g.kind==='move'?clampPreviewBox({...g.box,x:g.box.x+dx,y:g.box.y+dy},p.workspace):resizeBox(g.box,p.workspace,g.edge!,dx,dy));};
 const keyboardGeometry=(e:React.KeyboardEvent,edge?:Edge)=>{if(!['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home'].includes(e.key))return;e.preventDefault();e.stopPropagation();if(e.key==='Home'){commitBox(initialPreviewBox(p.workspace));return;}const d=e.shiftKey?50:10,dx=e.key==='ArrowLeft'?-d:e.key==='ArrowRight'?d:0,dy=e.key==='ArrowUp'?-d:e.key==='ArrowDown'?d:0;commitBox(edge?resizeBox(boxNow.current,p.workspace,edge,dx,dy):clampPreviewBox({...boxNow.current,x:boxNow.current.x+dx,y:boxNow.current.y+dy},p.workspace));};
 useEffect(()=>{if(decoded&&p.workspace.width>0&&p.workspace.height>0)p.remember?.({box,view,decoded,source:p.asset.previewPath,orientation:JSON.stringify(p.orientation)});},[box,view,decoded,p.asset.previewPath,p.orientation.rotation,p.orientation.mirrored]);
 const end=(e:ReactPointerEvent<HTMLElement>)=>{if(gesture.current?.pointer===e.pointerId)finish();};
 const pointer={onPointerMove:move,onPointerUp:end,onPointerCancel:end,onLostPointerCapture:end};
 return <div className="history-tree-preview-layer" hidden={p.workspace.width<=0||p.workspace.height<=0}><div ref={dialog} role="dialog" aria-modal="true" aria-label={p.title} className="history-tree-preview" data-testid="tree-preview" style={{left:box.x,top:box.y,width:box.width,height:box.height}} onKeyDown={e=>{e.stopPropagation();if(e.key==='Tab'){const nodes=[...e.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),[tabindex="0"]')].filter(n=>n.getBoundingClientRect().width>0),first=nodes[0],last=nodes.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus();}}}}>
  <header><span role="button" tabIndex={0} data-testid="tree-preview-move" aria-label={text('editor','tree.movePreview')} title={p.title} onPointerDown={e=>start(e,'move')} onKeyDown={e=>keyboardGeometry(e)} {...pointer}>{p.title}</span><button type="button" data-testid="tree-preview-close" aria-label={text('editor','tree.closePreview')} onClick={p.onClose}><Icon name="close"/></button></header>
  <div ref={stage} className="history-tree-preview-image" data-testid="tree-preview-image" data-scale={view.scale} data-mode={view.mode} data-pan-x={view.panX} data-pan-y={view.panY} tabIndex={0} aria-label={text('editor','tree.previewImage')} onPointerDown={e=>start(e,'pan')} onAuxClick={e=>{if(e.button===1)e.preventDefault();}} onKeyDown={e=>{if(e.key==='Home'){e.preventDefault();fitImage();}else if(e.key.startsWith('Arrow')){e.preventDefault();const d=e.shiftKey?50:10;commitView({...viewNow.current,mode:'manual',panX:viewNow.current.panX+(e.key==='ArrowLeft'?-d:e.key==='ArrowRight'?d:0),panY:viewNow.current.panY+(e.key==='ArrowUp'?-d:e.key==='ArrowDown'?d:0)});}}} {...pointer}>
   {!decoded&&!failed?<p role="status">{text('editor','tree.previewLoading')}</p>:null}
   {failed?<p role="alert">{text('editor','tree.previewMissing')}</p>:<img key={p.asset.previewPath} ref={image} draggable={false} src={convertFileSrc(p.asset.previewPath)} alt={p.title} onLoad={e=>{if(e.currentTarget!==image.current)return;const size={width:e.currentTarget.naturalWidth,height:e.currentTarget.naturalHeight};decodedNow.current=size;setDecoded(size);const memory=restored.current;restored.current=null;if(memory&&memory.decoded.width===size.width&&memory.decoded.height===size.height&&memory.view.mode==='manual')commitView(memory.view);else fitImage();}} onError={e=>{if(e.currentTarget===image.current)setFailed(true);}} style={{visibility:decoded?'visible':'hidden',position:'absolute',left:'50%',top:'50%',width:decoded?.width,height:decoded?.height,maxWidth:'none',maxHeight:'none',transform:`translate(-50%,-50%) translate(${view.panX}px,${view.panY}px) rotate(${p.orientation.rotation*90}deg) scaleX(${p.orientation.mirrored?-1:1}) scale(${view.scale})`}}/>}
   <div className="history-tree-preview-tools"><button type="button" aria-label={text('editor','tree.zoomOut')} disabled={!decoded||failed} onClick={()=>zoomAt(1/1.2,0,0)}><Icon name="minus"/></button><button type="button" aria-label={text('editor','tree.previewActual')} disabled={!decoded||failed} onClick={()=>commitView({scale:1,panX:0,panY:0,mode:'manual'})}>1:1</button><span>{Number((view.scale*100).toPrecision(3))}%</span><button type="button" aria-label={text('editor','tree.zoomIn')} disabled={!decoded||failed} onClick={()=>zoomAt(1.2,0,0)}><Icon name="plus"/></button><button type="button" aria-label={text('editor','tree.previewFit')} disabled={!decoded||failed} onClick={fitImage}><Icon name="fit"/></button></div>
   {decoded?<small className="history-tree-preview-resolution">{text('editor','tree.previewResolution',{width:decoded.width,height:decoded.height})}</small>:null}
  </div>
  <footer><button type="button" disabled={p.busy||!p.canHide} onClick={p.onVisibility}>{text('editor',p.hidden?'tree.restore':'tree.hide')}</button><button type="button" className="editor-primary" disabled={p.busy||failed||!decoded} onClick={()=>void p.onTake()}>{text('editor','tree.take')}</button></footer>
  {(['n','s','e','w','ne','nw','se','sw']as const).map(edge=><span key={edge} role="button" tabIndex={0} className={'history-tree-preview-resize is-'+edge} data-resize={edge} aria-label={text('editor',`tree.resize.${edge}`)} onPointerDown={e=>start(e,'resize',edge)} onKeyDown={e=>keyboardGeometry(e,edge)} {...pointer}/>)}
 </div></div>;
}
