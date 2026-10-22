import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {text as localeText} from "../../shared/i18n/Runtime";
import {useCurrentLocale} from "../../shared/i18n/Runtime";
import {useHelpPresentation} from '../help/HelpPresentation';
import {loadUiPreferences,saveUiPreferences} from '../../shared/UiPreferences';
import {useUiPreferences} from '../../shared/useUiPreferences';
import {Hint} from "../../shared/help/Hint";
import {flushSync} from 'react-dom';
import {SlideshowTransition,type SlideshowPhase} from './SlideshowTransition';
import {convertFileSrc} from "@tauri-apps/api/core";
import {invoke} from "../../shared/tauri/invoke";
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ChangeEvent, type MouseEvent, type PointerEvent as ReactPointerEvent, type SyntheticEvent } from 'react';
import type {
  SmartImageResizeFormat,
  SmartImageViewerFileActions,
  SmartImageViewerItem,
  SmartImageViewerWindowMode,
} from './ImageViewer.types';
import { smartImageViewerZoomPresets } from './ImageViewer.types';
import {nextViewerZoom,wheelViewerZoom} from './ViewerZoom';
import {SlideshowInterval} from './SlideshowInterval';
import { ImageViewerResizeDialog } from './ImageViewerResizeDialog';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import type { ImageFileMetadata } from '../explorer/types';
import { thumbnailService } from '../thumbnail/ThumbnailService';
import {HelpButton} from '../help/HelpButton';
import { Icon } from '../../shared/ui/Icon';
import {editorService} from '../photoEditor/EditorService';
import {messages} from '../../shared/i18n/ru';

interface SmartImageViewerProps {
  item: SmartImageViewerItem;
  allImages: SmartImageViewerItem[];
  windowMode: SmartImageViewerWindowMode;
  detailsOpen: boolean;
  fileActions: SmartImageViewerFileActions;
  onClose(): void;
  onWindowModeChange(mode: SmartImageViewerWindowMode): void;
  onToggleDetails(): void;
  onToggleFavorite(itemId: string): void | Promise<void>;
  onOpenImage(itemId: string): void;
  onImageDeleted?(deletedItemId: string, deletedPath: string, nextItemId: string | null): void;
  onTopBarMouseDown?(event: MouseEvent<HTMLElement>): void;
  onFullscreenChange?(enabled:boolean):Promise<void>;
}

interface DragState {
  isDragging: boolean;
  pointerId:number;
  node:HTMLDivElement;
  startX: number;
  startY: number;
  startPanX: number;
  startPanY: number;
}

interface ImageNaturalSize {
  width: number;
  height: number;
}

type ZoomMode = 'fit' | 'manual';
interface ViewerViewport {
  zoom:number; fitZoom:number; zoomMode:ZoomMode;
  pan:{x:number;y:number}; naturalSize:ImageNaturalSize|null; rotation:number;
  path:string; sourceRevision:string|number|undefined;
}

type SharePayload = {
  title?: string;
  text?: string;
  files?: File[];
};

type ShareCapableNavigator = Navigator & {
  canShare?: (payload: SharePayload) => boolean;
  share?: (payload: SharePayload) => Promise<void>;
};

const minZoom = 10;
const maxZoom = 800;

const fitPaddingPx = 18;

export function SmartImageViewer({
  item:requestedItem,
  allImages,
  windowMode,
  detailsOpen,
  fileActions,
  onClose,
  onWindowModeChange,
  onToggleDetails,
  onToggleFavorite,
  onOpenImage,
  onImageDeleted,
  onTopBarMouseDown,
  onFullscreenChange,
}: SmartImageViewerProps) {
  useCurrentLocale();
  const [item,setDisplayedItem]=useState(requestedItem);
  useEffect(()=>setDisplayedItem(previous=>previous.path===requestedItem.path?requestedItem:previous),[requestedItem]);
  const [slideshowSeconds,setSlideshowSeconds]=useState(3);
  const stageRef = useRef<HTMLDivElement | null>(null);
  const displayedImageRef = useRef<HTMLImageElement | null>(null);
  const menuWrapRef = useRef<HTMLDivElement | null>(null);
  const menuRef=useRef<HTMLDivElement|null>(null);
  const menuWasOpen=useRef(false);
  const slideshowTimerRef = useRef<number | null>(null);
  const fitFrameRef = useRef<number | null>(null);
  const fitGenerationRef = useRef(0);
  const fitModeRef = useRef(true);
  const [viewport,setViewport]=useState<ViewerViewport>({zoom:100,fitZoom:100,zoomMode:'fit',pan:{x:0,y:0},naturalSize:null,rotation:0,path:item.path,sourceRevision:item.sourceRevision});
  const viewNow=useRef(viewport);
  const {zoom,fitZoom,zoomMode,naturalSize,rotation,pan}=viewport;
  // The ref is the synchronous owner, including consecutive events before React paints.
  function commitView(patch:Partial<ViewerViewport>){
    const next={...viewNow.current,...patch};viewNow.current=next;setViewport(next);
  }
  const [sourceSize,setSourceSize]=useState<{path:string;width:number;height:number}|null>(null);
  const [drag, setDrag] = useState<DragState | null>(null);
  const dragRef=useRef<DragState|null>(null);
  const [menuOpen, setMenuOpen] = useState(false);
  const [resizeOpen, setResizeOpen] = useState(false);
  useEffect(() => {
    if (!resizeOpen) return;
    return () => { requestAnimationFrame(() => menuWrapRef.current?.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]')?.focus()); };
  }, [resizeOpen]);
  const [slideshowRunning, setSlideshowRunning] = useState(false);
  const [editorOpening,setEditorOpening]=useState(false);
  const [slideshowActive,setSlideshowActive]=useState(false);
  const preferences=useUiPreferences();
  const [ordinaryChromeHidden,setChromeHidden]=useState(()=>loadUiPreferences().viewerOnlyPhoto);
  const help=useHelpPresentation(),chromeHidden=ordinaryChromeHidden&&!help.open;
  const imageFitPadding=slideshowActive||chromeHidden?0:fitPaddingPx;

  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [preview, setPreview] = useState<{ path: string; source: string | null; error?: string; note?: string; loading?: boolean }>({ path: '', source: null });

  const currentImageIndex = useMemo(() => allImages.findIndex((candidate) => candidate.id === item.id), [allImages, item.id]);
  const imageSrc = preview.path === item.path ? preview.source : null;
  const isMinimized = windowMode === 'minimized';
  const verifiedSource=sourceSize?.path===item.path?sourceSize:null;
  const displayDimensions=verifiedSource?`${verifiedSource.width} × ${verifiedSource.height}`:item.dimensions??localeText("viewer", "ui.SmartImageViewer.94e4d9ab2089b729");

  function calculateFitZoom(size: ImageNaturalSize): number | null {
    const stage = stageRef.current;
    if (!stage || size.width <= 0 || size.height <= 0) return null;

    const rect = stage.getBoundingClientRect();
    const availableWidth = Math.max(0, rect.width - imageFitPadding);
    const availableHeight = Math.max(0, rect.height - imageFitPadding);
    const sideways = viewNow.current.rotation % 180 !== 0;
    const scale = Math.min(availableWidth / (sideways ? size.height : size.width), availableHeight / (sideways ? size.width : size.height));

    return scale>0?Math.min(scale * 100,maxZoom):null;
  }

  function applyFitZoom(size: ImageNaturalSize | null = viewNow.current.naturalSize): void {
    if (!size) return;
    const nextZoom = calculateFitZoom(size);
    if (!nextZoom) return;

    commitView({fitZoom:nextZoom,zoom:nextZoom,pan:{x:0,y:0}});
  }

  function scheduleFitZoom(size: ImageNaturalSize | null = viewNow.current.naturalSize): void {
    if (!size || !fitModeRef.current) return;
    cancelFitZoom();
    const generation = fitGenerationRef.current,source=viewNow.current;
    fitFrameRef.current = window.requestAnimationFrame(() => {
      if (generation !== fitGenerationRef.current || !fitModeRef.current || source.path!==viewNow.current.path || source.sourceRevision!==viewNow.current.sourceRevision) return;
      fitFrameRef.current = window.requestAnimationFrame(() => {
        fitFrameRef.current = null;
        if (generation !== fitGenerationRef.current || !fitModeRef.current || source.path!==viewNow.current.path || source.sourceRevision!==viewNow.current.sourceRevision) return;
        applyFitZoom(size);
      });
    });
  }

  function cancelFitZoom(): void {
    fitGenerationRef.current += 1;
    if (fitFrameRef.current !== null) window.cancelAnimationFrame(fitFrameRef.current);
    fitFrameRef.current = null;
  }

  function setManualZoom(value: number): void {
    fitModeRef.current = false;
    cancelFitZoom();
    const cleanZoom = clamp(value, minZoom, maxZoom);
    commitView({zoomMode:'manual',zoom:cleanZoom});

  }

  function setFitMode(): void {
    fitModeRef.current = true;
    commitView({zoomMode:'fit',pan:{x:0,y:0}});
    applyFitZoom();
    scheduleFitZoom();
  }

  useEffect(() => {
    const controller=new AbortController(),path=requestedItem.path;
    setPreview(previous=>({...previous,loading:true,error:undefined}));
    const preload=(url:string)=>new Promise<ImageNaturalSize>((resolve,reject)=>{
      const image=new Image();const cleanup=()=>{image.onload=null;image.onerror=null;controller.signal.removeEventListener('abort',abort);};
      const abort=()=>{cleanup();reject(new DOMException('Aborted','AbortError'));};
      image.onload=()=>{const size={width:image.naturalWidth,height:image.naturalHeight};cleanup();resolve(size);};image.onerror=()=>{cleanup();reject(Error(localeText("viewer", "ui.SmartImageViewer.bd03d1a276e5df39")));};controller.signal.addEventListener('abort',abort,{once:true});image.src=url;
    });
    void (async()=>{try{
      let source=requestedItem.previewUrl||buildImageSrc(path)+(requestedItem.sourceRevision?`?revision=${encodeURIComponent(requestedItem.sourceRevision)}`:""),note:string|undefined,metadata:ImageFileMetadata|null=null;
      if(isTauriRuntime()){
        metadata=await invoke<ImageFileMetadata>('inspect_image_metadata',{path});if(controller.signal.aborted)return;
        if(metadata.width&&metadata.height&&metadata.width*metadata.height>32_000_000)throw Error(localeText("viewer", "ui.SmartImageViewer.fb12b277023b168c"));
        if(/\.(?:heic|heif)$/i.test(path))throw Error(localeText("viewer", "ui.SmartImageViewer.609408918eec4f1b"));
        if(/\.(?:tif|tiff)$/i.test(path)){const thumbnail=await thumbnailService.getOrCreateThumbnail({imagePath:path,width:1024,height:1024,priority:true,signal:controller.signal});source=thumbnail.thumbnailUrl;note=localeText("viewer", "ui.SmartImageViewer.eb2fbabb653a2b6b");}
      }
      if(controller.signal.aborted)return;const size=await preload(source);if(controller.signal.aborted)return;
      if(metadata?.width&&metadata?.height)setSourceSize({path,width:metadata.width,height:metadata.height});
      cancelFitZoom();fitModeRef.current=true;commitView({path,sourceRevision:requestedItem.sourceRevision,zoomMode:'fit',pan:{x:0,y:0},rotation:0,naturalSize:size});setMenuOpen(false);setStatusMessage(null);
      const rect=stageRef.current?.getBoundingClientRect();if(rect&&rect.width>imageFitPadding&&rect.height>imageFitPadding){const fit=Math.min(maxZoom,Math.max(1,rect.width-imageFitPadding)/size.width*100,Math.max(1,rect.height-imageFitPadding)/size.height*100);commitView({fitZoom:fit,zoom:fit});}
      setDisplayedItem(requestedItem);setPreview({path,source,note,loading:false});
    }catch(error){if(!controller.signal.aborted){setDisplayedItem(requestedItem);setPreview({path,source:null,error:String(error),loading:false});}}})();
    return()=>controller.abort();
  },[requestedItem.path,requestedItem.previewUrl,requestedItem.sourceRevision]);

  useLayoutEffect(()=>{
    const menu=menuRef.current,button=menuWrapRef.current?.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]');
    if(menu&&button){
      const rect=button.getBoundingClientRect(),width=menu.getBoundingClientRect().width;
      menu.style.left=`${Math.max(8,Math.min(rect.right-width,window.innerWidth-width-8))}px`;
      menu.style.top=`${rect.bottom+6}px`;
      menu.style.maxHeight=`${Math.max(32,window.innerHeight-rect.bottom-14)}px`;
      if(!menuWasOpen.current)menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
    }else if(menuWasOpen.current&&!resizeOpen)button?.focus();
    menuWasOpen.current=menuOpen;
  });

  useEffect(() => {
    if (!menuOpen) return;

    const handlePointerDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (target && menuWrapRef.current?.contains(target)) return;
      setMenuOpen(false);
    };

    document.addEventListener('pointerdown', handlePointerDown, true);
    return () => document.removeEventListener('pointerdown', handlePointerDown, true);
  }, [menuOpen]);

  useEffect(() => {
    if (zoomMode !== 'fit' || !naturalSize) return;

    const updateFit = () => scheduleFitZoom(naturalSize);
    const animationFrameId = window.requestAnimationFrame(updateFit);
    const stage = stageRef.current;
    const resizeObserver = stage && 'ResizeObserver' in window ? new ResizeObserver(updateFit) : null;

    if (stage && resizeObserver) resizeObserver.observe(stage);
    window.addEventListener('resize', updateFit);

    return () => {
      window.cancelAnimationFrame(animationFrameId);
      cancelFitZoom();
      resizeObserver?.disconnect();
      window.removeEventListener('resize', updateFit);
    };
  }, [detailsOpen, naturalSize, windowMode, zoomMode, rotation,imageFitPadding]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if(document.documentElement.dataset.helpActive==='true')return;
      const target = event.target as HTMLElement | null;
      if (target?.closest('input, textarea, select, [contenteditable="true"]')&&!(event.key==='Escape'&&resizeOpen)) return;
      if(slideshowActive&&(event.code==='Space'||event.key===' ')){
        event.preventDefault();event.stopPropagation();
        if(!event.repeat)toggleSlideshowPlayback();
        return;
      }
      if (event.key === 'Escape') {
        event.preventDefault();event.stopPropagation();
        if (resizeOpen) {
          setResizeOpen(false);
          return;
        }
        if (menuOpen) {
          setMenuOpen(false);
          return;
        }
        if(slideshowActive){void exitSlideshow();return;}
        if(chromeHidden){setChromeHidden(false);return;}
        onClose();
        return;
      }

      if (!resizeOpen && !menuOpen && event.key === 'ArrowRight') {
        event.preventDefault();
        openAdjacentImage(1);
      }

      if (!resizeOpen && !menuOpen && event.key === 'ArrowLeft') {
        event.preventDefault();
        openAdjacentImage(-1);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [currentImageIndex, allImages, menuOpen, onClose, resizeOpen,slideshowActive,chromeHidden,requestedItem.id]);

  useEffect(() => {
    if (!slideshowRunning||help.open) {
      if (slideshowTimerRef.current) {
    window.clearTimeout(slideshowTimerRef.current);
        slideshowTimerRef.current = null;
      }
      return;
    }

    if(preview.path!==item.path||preview.loading)return;
    slideshowTimerRef.current = window.setTimeout(() => openAdjacentImage(1), slideshowSeconds*1000);
    return () => {
      if (slideshowTimerRef.current) {
        window.clearTimeout(slideshowTimerRef.current);
        slideshowTimerRef.current = null;
      }
    };
  }, [slideshowRunning, help.open, currentImageIndex, allImages,item.path,preview.path,preview.loading,slideshowSeconds]);

  const helpOwner=useRef<string|null>(null);
  useEffect(()=>{
    if(help.open){if(helpOwner.current===null)helpOwner.current=requestedItem.path;else if(helpOwner.current!==requestedItem.path)setSlideshowRunning(false);}
    else {if(helpOwner.current!==null&&helpOwner.current!==requestedItem.path)setSlideshowRunning(false);helpOwner.current=null;}
  },[help.open,requestedItem.path]);

  const normalView=useRef<typeof viewNow.current|null>(null);
  const fullscreenCallback=useRef(onFullscreenChange);fullscreenCallback.current=onFullscreenChange;
  const transitionHandler=useRef<(phase:SlideshowPhase)=>void>(()=>{});
  transitionHandler.current=phase=>{
    if(phase==='entering'){
      finishPan();
      normalView.current={...viewNow.current};
      cancelFitZoom();fitModeRef.current=true;
      flushSync(()=>{setSlideshowActive(true);setSlideshowRunning(false);commitView({zoomMode:'fit',pan:{x:0,y:0}});setDrag(null);setMenuOpen(false);});
    }else if(phase==='show'){
      setSlideshowRunning(slideshowController.current!.desired);
    }else if(phase==='exiting'){
      if(slideshowTimerRef.current!==null)clearTimeout(slideshowTimerRef.current);
      slideshowTimerRef.current=null;setSlideshowRunning(false);
    }else{
      const previous=normalView.current,current=viewNow.current;
      const restore=previous&&previous.path===current.path&&previous.sourceRevision===current.sourceRevision?previous:null;
      fitModeRef.current=!restore||restore.zoomMode==='fit';cancelFitZoom();
      flushSync(()=>{
        setSlideshowActive(false);setSlideshowRunning(false);
        commitView({pan:restore?.pan??{x:0,y:0},zoomMode:restore?.zoomMode??'fit',...(restore?{zoom:restore.zoom,fitZoom:restore.fitZoom}:{})});
      });
      if(fitModeRef.current&&current.naturalSize){
        const rect=stageRef.current?.getBoundingClientRect(),size=current.naturalSize,sideways=current.rotation%180!==0;
        if(rect){const fit=Math.min(maxZoom,Math.min(Math.max(1,rect.width-(chromeHidden?0:fitPaddingPx))/(sideways?size.height:size.width),Math.max(1,rect.height-(chromeHidden?0:fitPaddingPx))/(sideways?size.width:size.height))*100);flushSync(()=>{commitView({zoom:fit,fitZoom:fit});});}
      }
      normalView.current=null;
    }
  };
  const slideshowController=useRef<SlideshowTransition|null>(null);
  const createSlideshowController=()=>new SlideshowTransition(enabled=>fullscreenCallback.current?.(enabled)??Promise.resolve(),phase=>transitionHandler.current(phase));
  if(!slideshowController.current)slideshowController.current=createSlideshowController();
  useEffect(()=>{const live=slideshowController.current??(slideshowController.current=createSlideshowController());return()=>{live.dispose();if(slideshowController.current===live)slideshowController.current=null;};},[]);
  const startSlideshow=()=>slideshowController.current!.request(true).catch(error=>setStatusMessage(localeText("viewer", "ui.SmartImageViewer.5b24e8ed3584f8d6", {"value1": (String(error))})));
  const exitSlideshow=()=>{
    if(slideshowTimerRef.current!==null)clearTimeout(slideshowTimerRef.current);
    slideshowTimerRef.current=null;setSlideshowRunning(false);
    return slideshowController.current!.request(false).catch(error=>setStatusMessage(localeText("viewer", "ui.SmartImageViewer.b2975902742d2795", {"value1": (String(error))})));
  };
  const toggleSlideshowPlayback=()=>{
    if(slideshowController.current?.phase!=='show')return;
    if(slideshowTimerRef.current!==null)clearTimeout(slideshowTimerRef.current);
    slideshowTimerRef.current=null;setSlideshowRunning(value=>!value);
  };

  const openAdjacentImage = (direction: 1 | -1) => {
    if(slideshowTimerRef.current!==null)window.clearTimeout(slideshowTimerRef.current);slideshowTimerRef.current=null;
    if (allImages.length === 0) return;
    const requestedIndex=allImages.findIndex(candidate=>candidate.id===requestedItem.id);
    const currentIndex = requestedIndex >= 0 ? requestedIndex : 0;
    const nextIndex = (currentIndex + direction + allImages.length) % allImages.length;
    const nextItem = allImages[nextIndex];
    if (nextItem) {
      onOpenImage(nextItem.id);
    }
  };

  const handleImageLoad = (event: SyntheticEvent<HTMLImageElement>) => {
    const target = event.currentTarget;
    if(target!==displayedImageRef.current)return;
    const nextNaturalSize = {
      width: target.naturalWidth,
      height: target.naturalHeight,
    };

    commitView({naturalSize:nextNaturalSize});
    if (viewNow.current.zoomMode === 'fit') {
      scheduleFitZoom(nextNaturalSize);
    }
  };

  const handleWheel = (event: globalThis.WheelEvent) => {
    if((event.target as Element).closest('button,input,select,textarea,a,[role="button"]'))return;
    event.preventDefault();
    if(!event.deltaY)return;
    const image=displayedImageRef.current,stage=stageRef.current,current=viewNow.current;
    if(!image||!stage||!current.naturalSize)return;
    const stageRect=stage.getBoundingClientRect();
    const sideways=current.rotation%180!==0,size=current.naturalSize;
    const fitRect=current.zoomMode==='fit'?image.getBoundingClientRect():null;
    // Only CSS Fit needs a measurement. Manual zoom never feeds rounded DOM pixels back.
    const actualZoom=fitRect?fitRect.width/(sideways?size.height:size.width)*100:current.zoom;
    if(!(actualZoom>0))return;
    const pixels=event.deltaY*(event.deltaMode===1?16:event.deltaMode===2?stage.clientHeight:1);
    const next=wheelViewerZoom(actualZoom,pixels);
    if(next===actualZoom)return;
    const displayedPan=fitRect?{x:fitRect.left+fitRect.width/2-stageRect.left-stageRect.width/2,y:fitRect.top+fitRect.height/2-stageRect.top-stageRect.height/2}:current.pan;
    const halfWidth=(sideways?size.height:size.width)*actualZoom/200,halfHeight=(sideways?size.width:size.height)*actualZoom/200;
    const x=clamp(event.clientX-stageRect.left-stageRect.width/2,displayedPan.x-halfWidth,displayedPan.x+halfWidth);
    const y=clamp(event.clientY-stageRect.top-stageRect.height/2,displayedPan.y-halfHeight,displayedPan.y+halfHeight);
    const ratio=next/actualZoom;
    finishPan();fitModeRef.current=false;cancelFitZoom();
    commitView({zoomMode:'manual',zoom:next,pan:{x:x-(x-displayedPan.x)*ratio,y:y-(y-displayedPan.y)*ratio}});
  };
  const wheelHandler=useRef(handleWheel);wheelHandler.current=handleWheel;
  useEffect(()=>{
    const stage=stageRef.current;if(!stage)return;
    const wheel=(event:globalThis.WheelEvent)=>wheelHandler.current(event);
    stage.addEventListener('wheel',wheel,{passive:false});
    return()=>stage.removeEventListener('wheel',wheel);
  },[isMinimized]);

  function finishPan(){
    const current=dragRef.current;if(!current)return;
    dragRef.current=null;setDrag(null);
    if(current.node.hasPointerCapture(current.pointerId))current.node.releasePointerCapture(current.pointerId);
  }
  useEffect(()=>{
    const blur=()=>finishPan();window.addEventListener('blur',blur);
    return()=>{window.removeEventListener('blur',blur);const current=dragRef.current;dragRef.current=null;if(current?.node.hasPointerCapture(current.pointerId))current.node.releasePointerCapture(current.pointerId);};
  },[]);
  useEffect(()=>{finishPan();},[requestedItem.path]);
  const handleStagePointerDown=(event:ReactPointerEvent<HTMLDivElement>)=>{
    if(event.button!==0||viewNow.current.zoom<=viewNow.current.fitZoom+1||(event.target as Element).closest('button,input,select,textarea,[contenteditable="true"]'))return;
    event.preventDefault();event.currentTarget.setPointerCapture(event.pointerId);
    const next={isDragging:true,pointerId:event.pointerId,node:event.currentTarget,startX:event.clientX,startY:event.clientY,startPanX:viewNow.current.pan.x,startPanY:viewNow.current.pan.y};
    dragRef.current=next;setDrag(next);
  };
  const handleStagePointerMove=(event:ReactPointerEvent<HTMLDivElement>)=>{
    const current=dragRef.current;if(!current||current.pointerId!==event.pointerId)return;
    commitView({pan:{x:current.startPanX+event.clientX-current.startX,y:current.startPanY+event.clientY-current.startY}});
  };
  const handleStagePointerEnd=(event:ReactPointerEvent<HTMLDivElement>)=>{
    if(dragRef.current?.pointerId===event.pointerId)finishPan();
  };
  const handleStageDoubleClick=(event:MouseEvent<HTMLElement>)=>{
    if(!(event.target instanceof Element)||event.target.closest('button,input,select,textarea,a,[role="button"],[role="slider"],[role="menuitem"],[contenteditable="true"]'))return;
    event.preventDefault();event.stopPropagation();finishPan();
    if(slideshowActive)void exitSlideshow();else setChromeHidden(value=>!value);
  };

  const handleZoomPresetChange = (event: ChangeEvent<HTMLSelectElement>) => {
    const nextZoom = Number(event.target.value);
    if (Number.isFinite(nextZoom)) {
      setManualZoom(nextZoom);
    }
  };

  const runExternalAction = async (label: string, action: () => Promise<void>) => {
    try {
      setStatusMessage(`${label}…`);
      await action();
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.6a4d6bae9a44a12a", {"value1": (label)}));
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const handleSaveAs = async () => {
    setMenuOpen(false);

    if (!fileActions.chooseSaveOriginalImagePath || !fileActions.saveOriginalImageCopy) {
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.d2eb5ff37ead9b9c"));
      return;
    }

    try {
      const choice = await fileActions.chooseSaveOriginalImagePath({
        sourcePath: item.path,
        defaultFileName: item.name,
      });

      if (!choice.selectedPath) {
        setStatusMessage(localeText("viewer", "ui.SmartImageViewer.cd48cdca45e8a988"));
        return;
      }

      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.46f4b38538a78f8d"));
      const result = await fileActions.saveOriginalImageCopy({
        sourcePath: item.path,
        targetPath: choice.selectedPath,
      });

      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.1ed02aa10776f4d5", {"value1": (result.targetPath)}));
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const handleDeleteImage = async () => {
    setMenuOpen(false);

    if (!fileActions.deleteImage) {
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.171d400bbcbc5262"));
      return;
    }

    const confirmed = window.confirm(localeText("viewer", "ui.SmartImageViewer.b1bf9e7ae746c65e", {"value1": (item.name)}));
    if (!confirmed) return;

    const currentIndex = currentImageIndex >= 0 ? currentImageIndex : 0;
    const nextItem = allImages[currentIndex + 1] ?? allImages[currentIndex - 1] ?? null;

    try {
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.102496788585d6c4"));
      await fileActions.deleteImage(item.path);

      if (onImageDeleted) {
        onImageDeleted(item.id, item.path, nextItem?.id ?? null);
      } else if (nextItem) {
        onOpenImage(nextItem.id);
      }

      if (nextItem) {
        setStatusMessage(localeText("viewer", "ui.SmartImageViewer.f8d8a24e8969d8eb", {"value1": (nextItem.name)}));
        return;
      }

      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.dc4b9e486d4e5692"));
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const handlePrintImage = async () => {
    setMenuOpen(false);

    if (!fileActions.printImage) {
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.e4cbd8d5749ba82b"));
      return;
    }

    try {
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.dc2be23f95eec89b"));
      const result = await fileActions.printImage(item.path);
      setStatusMessage(result.action === 'printCancelled' ? localeText("viewer", "ui.SmartImageViewer.dd154d9c2d50e261") : localeText("viewer", "ui.SmartImageViewer.96fe39363cf2d7a4"));
    } catch (error) {
      setStatusMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const handleShareImage = async () => {
    setMenuOpen(false);

    if (fileActions.shareImage) {
      try {
        setStatusMessage(localeText("viewer", "ui.SmartImageViewer.53c0060c90a4b934"));
        await fileActions.shareImage(item.path);
        setStatusMessage(localeText("viewer", "ui.SmartImageViewer.837876186608a252"));
      } catch (error) {
        setStatusMessage(error instanceof Error ? error.message : String(error));
      }
      return;
    }

    if (!fileActions.readImageFileForShare) {
      await runExternalAction(localeText("viewer", "ui.SmartImageViewer.3473789716f120d1"), () => fileActions.revealInFileManager(item.path));
      return;
    }

    const shareNavigator = navigator as ShareCapableNavigator;
    if (!shareNavigator.share) {
      await runExternalAction(localeText("viewer", "ui.SmartImageViewer.3473789716f120d1"), () => fileActions.revealInFileManager(item.path));
      return;
    }

    try {
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.61e60cd40f8df3fb"));
      const fileData = await fileActions.readImageFileForShare?.(item.path);
      if (!fileData) {
        throw new Error(localeText("viewer", "ui.SmartImageViewer.3b3f57ffb4ac5724"));
      }

      const bytes = new Uint8Array(fileData.bytes);
      const file = new File([bytes], fileData.fileName, { type: fileData.mimeType });
      const payload: SharePayload = {
        title: item.name,
        text: item.name,
        files: [file],
      };

      if (shareNavigator.canShare && !shareNavigator.canShare(payload)) {
        await fileActions.revealInFileManager(item.path);
        setStatusMessage(localeText("viewer", "ui.SmartImageViewer.474a4c7432a7a465"));
        return;
      }

      await shareNavigator.share(payload);
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.7addd79aa3c4d924"));
    } catch (error) {
      setStatusMessage(error instanceof DOMException && error.name === 'AbortError'
        ? localeText("viewer", "ui.SmartImageViewer.fb93fb92949f5785") : error instanceof Error ? error.message : String(error));
    }
  };

  const handleSetDesktopWallpaper = async () => {
    setMenuOpen(false);

    if (!fileActions.setDesktopWallpaper) {
      setStatusMessage(localeText("viewer", "ui.SmartImageViewer.44a4644650b0ef19"));
      return;
    }

    await runExternalAction(localeText("viewer", "ui.SmartImageViewer.fe17e84f7eba2b36"), async () => {
      await fileActions.setDesktopWallpaper?.(item.path);
    });
  };

  if (isMinimized) {
    return (
      <section className="smart-image-viewer smart-image-viewer--minimized" aria-label={localeText("viewer", "ui.SmartImageViewer.905eee7cceb0ed89")}>
        <Hint message={{id:"viewer.open"}}><button type="button" onClick={() => onWindowModeChange('full')}>{localeText("viewer", "ui.SmartImageViewer.a5b1dcd911964c4e")}{' '}{item.name}</button></Hint>
        <Hint message={{id:"viewer.close"}}><button type="button" onClick={onClose} aria-label={localeText("viewer", "ui.SmartImageViewer.4d3864869f77f032")}><Icon name="close"/></button></Hint>
      </section>
    );
  }

  return (
    <section data-help-open={help.open?true:undefined} data-tour-id="viewer.overview" className={`smart-image-viewer smart-image-viewer--${windowMode}${chromeHidden?' smart-image-viewer--chrome-hidden':''}${slideshowActive?' smart-image-viewer--slideshow':''}`} aria-label={localeText("viewer", "ui.SmartImageViewer.1981f2cb96625efe")}>
      <header data-tour-id="viewer.top.overview"
        className="smart-image-viewer__topbar"
        onMouseDown={(event) => {
          if (event.button !== 0) return;
          const target = event.target as HTMLElement | null;
          if (target?.closest('button, input, select, textarea, a')) return;
          onTopBarMouseDown?.(event);
        }}
      >
        <div data-help-id="viewer-controls" className="smart-image-viewer__topbar-left">

          <Hint message={{id:"viewer.edit"}}><button data-tour-id="viewer.top.edit" type="button" aria-label={localeText("viewer", "ui.SmartImageViewer.032750800191f6f1")} title={localeText("viewer", "ui.SmartImageViewer.4df9e021660c2bfa")} disabled={editorOpening} onClick={()=>{setSlideshowRunning(false);if(slideshowTimerRef.current){window.clearTimeout(slideshowTimerRef.current);slideshowTimerRef.current=null;}setEditorOpening(true);void editorService.open(item.path).catch(e=>setStatusMessage(localeText("viewer", "ui.SmartImageViewer.abaf8422b281d24d", {"value1": (String(e))}))).finally(()=>setEditorOpening(false));}}><Icon name="image"/><span className="smart-image-viewer__button-label">{localeText("viewer", "ui.SmartImageViewer.26be2a21e8e605a0")}</span></button></Hint>
          <Hint message={{id:"viewer.rotate-left"}}><button data-tour-id="viewer.top.rotate" type="button" title={localeText("viewer", "ui.SmartImageViewer.677316aed22bc3a4")} onClick={() => { commitView({rotation:(viewNow.current.rotation+270)%360,pan:{x:0,y:0}}); }}><Icon name="rotateLeft" /></button></Hint>
          <Hint message={{id:"viewer.rotate-right"}}><button data-tour-id="viewer.top.rotate" type="button" title={localeText("viewer", "ui.SmartImageViewer.ac172cf5ca9aa23a")} onClick={() => { commitView({rotation:(viewNow.current.rotation+90)%360,pan:{x:0,y:0}}); }}><Icon name="rotateRight" /></button></Hint>
          <Hint message={{id:"viewer.trash"}}><button data-tour-id="viewer.top.delete" type="button" title={localeText("viewer", "ui.SmartImageViewer.0586893ecd3d6fb2")} onClick={() => { void handleDeleteImage(); }}><Icon name="trash" /></button></Hint>
          <Hint message={{id:"viewer.print"}}><button data-tour-id="viewer.top.print" type="button" title={localeText("viewer", "ui.SmartImageViewer.76bd31c86a5562ae")} onClick={() => { void handlePrintImage(); }}><Icon name="print" /></button></Hint>
          <Hint message={{id:"viewer.share"}}><button data-tour-id="viewer.top.share" type="button" title={localeText("viewer", "ui.SmartImageViewer.d265d7c8249ff3ce")} onClick={() => { void handleShareImage(); }}><Icon name="share" /></button></Hint>
          <div ref={menuWrapRef} className="smart-image-viewer__menu-wrap">
            <Hint message={{id:"viewer.more"}}><button data-tour-id="viewer.top.more" type="button" aria-label={localeText("viewer", "ui.SmartImageViewer.6a9dfb82301dd091")} aria-haspopup="menu" aria-expanded={menuOpen} onClick={() => setMenuOpen(!menuOpen)}><Icon name="more" /></button></Hint>
            {menuOpen ? (
              <div ref={menuRef} className="smart-image-viewer__menu" role="menu" onKeyDown={event=>{
                if(!['ArrowDown','ArrowUp','Home','End'].includes(event.key))return;
                event.preventDefault();
                const buttons=[...event.currentTarget.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')],index=buttons.indexOf(document.activeElement as HTMLButtonElement);
                buttons[event.key==='Home'?0:event.key==='End'?buttons.length-1:(index+(event.key==='ArrowDown'?1:-1)+buttons.length)%buttons.length]?.focus();
              }}>
                <Hint message={{id:"viewer.only-photo"}}><button type="button" role="menuitemcheckbox" aria-checked={preferences.viewerOnlyPhoto} onClick={()=>{
                  try{saveUiPreferences({viewerOnlyPhoto:!preferences.viewerOnlyPhoto});setMenuOpen(false);}catch(error){setStatusMessage(localeText("viewer", "ui.SmartImageViewer.de5ba455a20bf179")+String(error));}
                }}>{localeText("viewer", "ui.SmartImageViewer.d81e208cd27a237b")}{preferences.viewerOnlyPhoto?<Icon name="check"/>:null}</button></Hint>
          <Hint message={{id:"viewer.external-edit"}}><button type="button" role="menuitem" aria-label={localeText("viewer", "ui.SmartImageViewer.907654b513151edd")} title={localeText("viewer", "ui.SmartImageViewer.9d583c71f9a50d68")} onClick={() => { setMenuOpen(false); void runExternalAction(localeText("viewer", "ui.SmartImageViewer.66c80c9f48c74f8a"), () => fileActions.editExternal(item.path)); }}>
            <span className="smart-image-viewer__button-label">{localeText("viewer", "ui.SmartImageViewer.a17b69924936994e")}</span>
          </button></Hint>
                <Hint message={{id:"viewer.save-original"}}><button type="button" role="menuitem" onClick={() => { void handleSaveAs(); }}>
                  {localeText("viewer", "ui.SmartImageViewer.7ee54430a8c15799")}</button></Hint>
                <Hint message={{id:"viewer.resize"}}><button type="button" role="menuitem" onClick={() => { setMenuOpen(false); setResizeOpen(true); }}>
                  {localeText("viewer", "ui.SmartImageViewer.a90ce3de93d370be")}</button></Hint>
                <Hint message={{id:"common.reveal-file"}}><button type="button" role="menuitem" onClick={() => { setMenuOpen(false); void runExternalAction(localeText("viewer", "ui.SmartImageViewer.81321408a627a80e"), () => fileActions.revealInFileManager(item.path)); }}>
                  {localeText("viewer", "ui.SmartImageViewer.0c50a1867a30aeef")}</button></Hint>
                <div className="smart-image-viewer__menu-separator" />
                <span className="smart-image-viewer__submenu-label">{localeText("viewer", "ui.SmartImageViewer.d1a9f9837b182133")}</span>
                <Hint message={{id:"viewer.wallpaper"}}><button type="button" role="menuitem" onClick={() => { void handleSetDesktopWallpaper(); }}>
                  {localeText("viewer", "ui.SmartImageViewer.a27dc8d38c2c4f6f")}</button></Hint>
                <Hint message={{id:"viewer.lock-screen-unavailable"}}><button type="button" role="menuitem" disabled title={localeText("viewer", "ui.SmartImageViewer.616352723aada5d6")}>
                  {localeText("viewer", "ui.SmartImageViewer.06b9758f4857a7df")}</button></Hint>
              </div>
            ) : null}
          </div>
        </div>

        <strong className="smart-image-viewer__file-name" title={item.path}>{item.name}</strong>

        <div data-tour-id="viewer.top.window" className="smart-image-viewer__window-actions"><HelpButton side="left"/>
          <Hint message={{id:"common.minimize"}}><button type="button" title={localeText("viewer", "ui.SmartImageViewer.3f2105475bc16321")} onClick={() => onWindowModeChange('minimized')}><Icon name="minimize" /></button></Hint>
          <Hint message={{id:windowMode==='compact'?'common.maximize':'common.restore'}}><button type="button" title={windowMode === 'compact' ? localeText("viewer", "ui.SmartImageViewer.123c34759c48f360") : localeText("viewer", "ui.SmartImageViewer.f065d00725e3865e")} onClick={() => onWindowModeChange(windowMode === 'compact' ? 'full' : 'compact')}>
            <Icon name={windowMode === 'compact' ? 'maximize' : 'restore'} />
          </button></Hint>
          <Hint message={{id:"viewer.close"}}><button type="button" title={localeText("viewer", "ui.SmartImageViewer.7ebddb11a159d6cb")} onClick={onClose}><Icon name="close" /></button></Hint>
        </div>
      </header>

      <div className={detailsOpen&&!chromeHidden&&!slideshowActive ? 'smart-image-viewer__body smart-image-viewer__body--details-open' : 'smart-image-viewer__body'}>
        <main
          ref={stageRef}
          data-viewport-zoom={zoom} data-viewport-mode={zoomMode} data-viewport-path={viewport.path} data-viewport-rotation={rotation} data-viewport-pan-x={pan.x} data-viewport-pan-y={pan.y}
          data-tour-id="viewer.image" className={drag?.isDragging ? 'smart-image-viewer__stage smart-image-viewer__stage--dragging' : 'smart-image-viewer__stage'}
          onDoubleClick={handleStageDoubleClick}
          onPointerDown={handleStagePointerDown}
          onPointerMove={handleStagePointerMove}
          onPointerUp={handleStagePointerEnd}
          onPointerCancel={handleStagePointerEnd}
          onLostPointerCapture={handleStagePointerEnd}
        >
          {imageSrc ? (
            <div className="smart-image-viewer__image-frame">
              <img
                ref={displayedImageRef}
                key={item.path}
                data-help-id="viewer-image" className="smart-image-viewer__image"
                src={imageSrc}
                alt={item.name}
                draggable={false}
                onLoad={handleImageLoad}
                onError={() => setPreview((current) => current.path === item.path ? { ...current, source: null, error: localeText("viewer", "ui.SmartImageViewer.078efee05109ef0f") } : current)}
                style={{
                  ...(zoomMode==='fit'&&naturalSize?{
                    width:`min(calc(${rotation%180!==0?'100cqh':'100cqw'} - ${imageFitPadding}px), calc((${rotation%180!==0?'100cqw':'100cqh'} - ${imageFitPadding}px) * ${naturalSize.width/naturalSize.height}), ${naturalSize.width*maxZoom/100}px)`,height:'auto',maxWidth:'none',maxHeight:'none',transform:`rotate(${rotation}deg)`,
                  }:{width:naturalSize?.width,height:naturalSize?.height,maxWidth:'none',maxHeight:'none',transform:`translate(${pan.x}px, ${pan.y}px) rotate(${rotation}deg) scale(${zoom/100})`}),
                  transition:'none',
                }}
              />
            </div>
          ) : null}
          {preview.error?<div className="smart-image-viewer__preview-error" role="alert">{nativeDisplayText(preview.error??'')}</div>:null}
          {allImages.length > 1 && !slideshowActive ? <>
            <Hint message={{id:"viewer.prev"}}><button data-tour-id="viewer.image.neighbors" type="button" className="smart-image-viewer__navigate smart-image-viewer__navigate--previous" aria-label={localeText("viewer", "ui.SmartImageViewer.2e5751b41227d272")} onMouseDown={(event) => event.stopPropagation()} onClick={() => openAdjacentImage(-1)}><Icon name="left" /></button></Hint>
            <Hint message={{id:"viewer.next"}}><button data-tour-id="viewer.image.neighbors" type="button" className="smart-image-viewer__navigate smart-image-viewer__navigate--next" aria-label={localeText("viewer", "ui.SmartImageViewer.139b27976a9821e8")} onMouseDown={(event) => event.stopPropagation()} onClick={() => openAdjacentImage(1)}><Icon name="right" /></button></Hint>
          </> : null}
          {statusMessage ? <div className="smart-image-viewer__status-message">{statusMessage}</div> : null}
          {preview.path === item.path && preview.note ? <div className="smart-image-viewer__preview-note" role="status">{preview.note}</div> : null}
        </main>

        {detailsOpen&&!chromeHidden&&!slideshowActive ? (
          <aside className="smart-image-viewer__details" aria-label={localeText("viewer", "ui.SmartImageViewer.6d9f40ac51e66a97")}>
            <header>
              <strong>{localeText("viewer", "ui.SmartImageViewer.7790f1f1c951e9e2")}</strong>
              <Hint message={{id:"viewer.details-close"}}><button type="button" onClick={onToggleDetails} aria-label={localeText("viewer", "ui.SmartImageViewer.a14318920a4a5014")}><Icon name="close" /></button></Hint>
            </header>
            <DefinitionList item={item} displayDimensions={displayDimensions} />
          </aside>
        ) : null}
      </div>

      <footer data-tour-id="viewer.bottom.overview" className="smart-image-viewer__bottombar">
        <div className="smart-image-viewer__bottom-left">
          <Hint message={{id:item.isFavorite?'common.favorite-remove':'common.favorite-add'}}><button data-tour-id="viewer.bottom.favorite" type="button" title={localeText("viewer", "ui.SmartImageViewer.a20e6f45e888f059")} aria-pressed={Boolean(item.isFavorite)} onClick={() => { void onToggleFavorite(item.id); }}>
            <Icon name="star" filled={Boolean(item.isFavorite)} />
          </button></Hint>
          <Hint message={{id:"viewer.details"}}><button data-tour-id="viewer.bottom.details" type="button" title={localeText("viewer", "ui.SmartImageViewer.54fb08a004c5cd0f")} onClick={onToggleDetails}><Icon name="info" /></button></Hint>
          <span>{displayDimensions}</span>
          <span>{item.sizeLabel ?? formatBytes(item.sizeBytes)}</span>
        </div>

        <div className="smart-image-viewer__bottom-center">
          <SlideshowInterval value={slideshowSeconds} onChange={setSlideshowSeconds}/>
          <Hint message={{id:"viewer.slideshow"}}><button data-tour-id="viewer.bottom.slideshow" type="button" className="smart-image-viewer__slideshow" title={localeText("viewer", "ui.SmartImageViewer.912e09290d7e9bd9")} onClick={() => void startSlideshow()}>
            <Icon name={slideshowRunning ? 'pause' : 'play'} />
          </button></Hint>
        </div>

        <div className="smart-image-viewer__bottom-right">

          <Hint message={{id:"viewer.actual"}}><button data-tour-id="viewer.bottom.actual-size" type="button" title={localeText("viewer", "ui.SmartImageViewer.2d0ec2a7000514f1")} onClick={() => setManualZoom(100)}>1:1</button></Hint>
          <Hint message={{id:"viewer.zoom"}} supplement={{id:"viewer.pan"}}><label data-tour-id="viewer.bottom.zoom" data-help-id="viewer-scale" className="smart-image-viewer__zoom-select-label">
            <span>{localeText("viewer", "ui.SmartImageViewer.cf536d72e2ca6724")}</span>
            <select value="" onChange={handleZoomPresetChange} aria-label={localeText("viewer", "ui.SmartImageViewer.631b89a90367d6fb")}>
              <option value="" disabled>{zoomMode === 'fit' ? localeText("viewer", "ui.SmartImageViewer.e029548473078857", {"value1": (Number(zoom.toFixed(1)))}) : `${Number(zoom.toFixed(1))}%`}</option>
              {smartImageViewerZoomPresets.map((candidate) => (
                <option key={candidate} value={candidate}>{candidate}%</option>
              ))}
            </select>
          </label></Hint>
          <Hint message={{id:"viewer.zoom-out"}}><button data-tour-id="viewer.bottom.zoom" type="button" title={localeText("viewer", "ui.SmartImageViewer.6bf8b7b965fe3e9f")} onClick={() => {const next=nextViewerZoom(viewNow.current.zoom,-1);if(next!==viewNow.current.zoom)setManualZoom(next);}}><Icon name="minus" /></button></Hint>
          <Hint message={{id:"viewer.zoom"}}><input data-tour-id="viewer.bottom.zoom" type="range" min={zoomMode==='fit'?Math.min(minZoom,fitZoom):minZoom} max={maxZoom} step={zoomMode==='fit'?'any':5} value={zoom} onChange={(event: ChangeEvent<HTMLInputElement>) => setManualZoom(Math.round(Number(event.target.value)/5)*5)} /></Hint>
          <Hint message={{id:"viewer.zoom-in"}}><button data-tour-id="viewer.bottom.zoom" type="button" title={localeText("viewer", "ui.SmartImageViewer.ea85e5bba139dc87")} onClick={() => setManualZoom(nextViewerZoom(viewNow.current.zoom,1))}><Icon name="plus" /></button></Hint>
          <Hint message={{id:"viewer.fit"}} supplement={{id:"viewer.chrome-toggle"}}><button data-tour-id="viewer.bottom.fit" type="button" className={zoomMode === 'fit' ? 'smart-image-viewer__fit-button active' : 'smart-image-viewer__fit-button'} title={localeText("viewer", "ui.SmartImageViewer.dc17e0845a56a712")} onClick={setFitMode}>
            <Icon name="fit" />
          </button></Hint>
        </div>
      </footer>
      {chromeHidden&&!slideshowActive?<Hint message={{id:"viewer.restore-panels"}}><button type="button" className="slideshow-close slideshow-overlay-button" aria-label={localeText("viewer", "ui.SmartImageViewer.aaa1ef6c43111753")} onClick={()=>setChromeHidden(false)}><Icon name="close"/></button></Hint>:null}
      {slideshowActive?<><Hint message={{id:"viewer.slideshow-exit"}}><button type="button" className="slideshow-close slideshow-overlay-button" aria-label={localeText("viewer", "ui.SmartImageViewer.2155a06d501cfeea")} onClick={()=>void exitSlideshow()}><Icon name="close"/></button></Hint>
       <div data-tour-id="viewer.slideshow.controls" className="slideshow-controls"><SlideshowInterval value={slideshowSeconds} onChange={setSlideshowSeconds}/><Hint message={{id:"viewer.prev"}}><button data-tour-id="viewer.image.neighbors" type="button" className="slideshow-overlay-button" aria-label={localeText("viewer", "ui.SmartImageViewer.2e5751b41227d272")} onClick={()=>openAdjacentImage(-1)}><Icon name="left"/></button></Hint>
       <Hint message={{id:slideshowRunning?'viewer.slideshow-pause':'viewer.slideshow-resume'}}><button type="button" className="slideshow-overlay-button" aria-label={slideshowRunning?localeText("viewer", "ui.SmartImageViewer.e1a12c79d864d351"):localeText("viewer", "ui.SmartImageViewer.8a5356f1f8554c4a")} onClick={toggleSlideshowPlayback}><Icon name={slideshowRunning?'pause':'play'}/></button></Hint>
       <Hint message={{id:"viewer.next"}}><button data-tour-id="viewer.image.neighbors" type="button" className="slideshow-overlay-button" aria-label={localeText("viewer", "ui.SmartImageViewer.139b27976a9821e8")} onClick={()=>openAdjacentImage(1)}><Icon name="right"/></button></Hint></div></>:null}

      {resizeOpen ? (
        <ImageViewerResizeDialog
          item={{...item,dimensions:verifiedSource?`${verifiedSource.width} × ${verifiedSource.height}`:undefined}}
          fileActions={fileActions}
          onClose={() => setResizeOpen(false)}
          onSaved={(targetPath) => setStatusMessage(localeText("viewer", "ui.SmartImageViewer.3fb6118757421ac3", {"value1": (targetPath)}))}
        />
      ) : null}
    </section>
  );
}

function DefinitionList({ item, displayDimensions }: { item: SmartImageViewerItem; displayDimensions: string }) {
  useCurrentLocale();
  const rows = [
    [localeText("viewer", "ui.SmartImageViewer.c12c1f3c84088cf6"), item.name],
    [localeText("viewer", "ui.SmartImageViewer.56f1f2d80f05b3d4"), item.path],
    [localeText("viewer", "ui.SmartImageViewer.f457a1e090322076"), item.sizeLabel ?? formatBytes(item.sizeBytes)],
    [localeText("viewer", "ui.SmartImageViewer.1d47df3b38adc56a"), displayDimensions],
    [localeText("viewer", "ui.SmartImageViewer.711e3a8a187a87db"), item.extension ? item.extension.toUpperCase() : localeText("viewer", "ui.SmartImageViewer.d6e3a29c7254b586")],
    [localeText("viewer", "ui.SmartImageViewer.dafb7f9c0e139798"), item.dateTaken || localeText("viewer", "ui.SmartImageViewer.4035388bb782d6f9")],
    [localeText("viewer", "ui.SmartImageViewer.ddf3ca1633132f0c"), item.status?messages().explorer.statusLabels[item.status as keyof ReturnType<typeof messages>['explorer']['statusLabels']]??localeText("viewer", "ui.SmartImageViewer.87d37a0d83b4ea84"):localeText("viewer", "ui.SmartImageViewer.87d37a0d83b4ea84")],
    [localeText("viewer", "ui.SmartImageViewer.eecb7ec2971022bc"), item.isFavorite ? localeText("viewer", "ui.SmartImageViewer.b5316310f27cc22f") : localeText("viewer", "ui.SmartImageViewer.54608e96cbcc08f1")],
    [localeText("viewer", "ui.SmartImageViewer.9af454cb0da9fdc9"), item.tags?.length ? item.tags.join(', ') : localeText("viewer", "ui.SmartImageViewer.d05c0df2cff354d5")],
    [localeText("viewer", "ui.SmartImageViewer.39339e66764b4fc4"), item.collections?.length ? item.collections.join(', ') : localeText("viewer", "ui.SmartImageViewer.54608e96cbcc08f1")],
    [localeText("viewer", "ui.SmartImageViewer.bb0f32c51779f4e6"), item.description?.trim() ? item.description : localeText("viewer", "ui.SmartImageViewer.3114931a8d9cab7e")],
  ];

  return (
    <dl>
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function normalizeViewerPath(path: string): string {
  const slash = String.fromCharCode(92);
  const prefix = slash + slash + '?' + slash;
  let cleanPath = path.trim();
  if (cleanPath.toLowerCase().startsWith(prefix.toLowerCase())) {
    cleanPath = cleanPath.slice(prefix.length);
  }
  return cleanPath;
}

function buildImageSrc(path: string): string {
  const cleanPath = normalizeViewerPath(path);
  if (!cleanPath) return '';
  try {
    return convertFileSrc(cleanPath);
  } catch {
    return '';
  }
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, value));
}

function formatBytes(value?: number): string {
  if (!value || !Number.isFinite(value) || value < 0) return localeText("viewer", "ui.SmartImageViewer.da1c4ec5e7ee34dd");
  if (value < 1024) return localeText("viewer", "ui.SmartImageViewer.2d7b05fbda0356c7", {"value1": (value)});
  if (value < 1024 * 1024) return localeText("viewer", "ui.SmartImageViewer.380593f595ace7ee", {"value1": ((value / 1024).toFixed(1))});
  if (value < 1024 * 1024 * 1024) return localeText("viewer", "ui.SmartImageViewer.524e39af57186fbb", {"value1": ((value / 1024 / 1024).toFixed(1))});
  return localeText("viewer", "ui.SmartImageViewer.3621bb04e496ea39", {"value1": ((value / 1024 / 1024 / 1024).toFixed(1))});
}
