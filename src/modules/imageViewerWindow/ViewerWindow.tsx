import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {Icon} from '../../shared/ui/Icon';
import { useEffect, useMemo, useRef, useState } from 'react';
import { currentMonitor, getCurrentWindow } from '@tauri-apps/api/window';
import {invoke} from "../../shared/tauri/invoke";
import { SmartImageViewer } from '../imageViewer/SmartImageViewer';
import { tauriSmartImageViewerFileActions } from '../imageViewer/TauriSmartImageViewerFileService';
import type { SmartImageViewerWindowMode } from '../imageViewer/ImageViewer.types';
import { collectionService } from '../collections/CollectionService';
import { broadcastImageFavoriteUpdated, listenForImageFavoriteUpdates } from './favoriteSync';
import type { ViewerSessionPayload } from './viewerSession';

async function waitForViewerClient(target:{width:number;height:number}):Promise<void>{
  await new Promise<void>((resolve,reject)=>{
    let frame=0,stable=0;
    const deadline=window.setTimeout(()=>{cancelAnimationFrame(frame);reject(new Error(localeText("main", "ui.ViewerWindow.92bf27b642f58563")));},2000);
    const tick=()=>{
      const matches=Math.abs(innerWidth*devicePixelRatio-target.width)<=2&&Math.abs(innerHeight*devicePixelRatio-target.height)<=2;
      stable=matches?stable+1:0;
      if(stable>=2){clearTimeout(deadline);resolve();}else frame=requestAnimationFrame(tick);
    };
    frame=requestAnimationFrame(tick);
  });
}

interface ViewerWindowProps {
  session: ViewerSessionPayload;
}

export function ViewerWindow({ session: initialSession }: ViewerWindowProps) {
  useCurrentLocale();
  const currentWindow = getCurrentWindow();
  const [session, setSession] = useState<ViewerSessionPayload>(initialSession);
  const [windowMode, setWindowMode] = useState<SmartImageViewerWindowMode>('full');
  const [detailsOpen, setDetailsOpen] = useState(false);
  const beforeFullscreen=useRef<{fullscreen:boolean;client:{width:number;height:number}}|null>(null);
  const changeFullscreen=async(enabled:boolean)=>{
    if(enabled){
      const previous={fullscreen:await currentWindow.isFullscreen(),client:await currentWindow.innerSize()};
      beforeFullscreen.current=previous;
      try{
        const monitor=await currentMonitor();
        if(!previous.fullscreen)await currentWindow.setFullscreen(true);
        await waitForViewerClient(monitor?.size??previous.client);
      }catch(error){
        await currentWindow.setFullscreen(previous.fullscreen).catch(()=>{});
        beforeFullscreen.current=null;throw error;
      }
    }else{
      const previous=beforeFullscreen.current;
      if(!previous)return;
      // Tao restores WINDOWPLACEMENT itself; no maximize/unmaximize sequence.
      try{
        if(!previous.fullscreen)await currentWindow.setFullscreen(false);
        await waitForViewerClient(previous.client);
      }catch(error){if(!previous.fullscreen)await currentWindow.setFullscreen(true).catch(()=>{});throw error;}
      beforeFullscreen.current=null;
      setWindowMode(await currentWindow.isMaximized()?'full':'compact');
    }
  };
  const navigationRevisionRef = useRef(initialSession.navigationRevision ?? 0);

  useEffect(() => {
    const revision = initialSession.navigationRevision ?? 0;
    if (revision < navigationRevisionRef.current) return;
    navigationRevisionRef.current = revision;
    setSession(initialSession);
  }, [initialSession]);
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const sync = () => { void currentWindow.isMaximized().then(maximized => {
      if (!cancelled&&beforeFullscreen.current===null) setWindowMode(maximized ? 'full' : 'compact');
    }); };
    sync();
    void currentWindow.onResized(sync).then(cleanup => {
      if (cancelled) cleanup(); else unlisten = cleanup;
    });
    return () => { cancelled = true; unlisten?.(); };
  }, []);

  useEffect(() => {
    setSession((current) => {
      if (current.images.length === 0) return current;
      if (current.images.some((item) => item.id === current.activeImageId)) return current;
      return { ...current, activeImageId: current.images[0]?.id ?? '' };
    });
  }, [session.activeImageId, session.images]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;

    void listenForImageFavoriteUpdates((result) => {
      setSession((current) => updateSessionFavoriteState(current, result));
    }).then((cleanup) => {
      if (disposed) cleanup?.(); else unlisten = cleanup;
    });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  useEffect(() => {
    const activeItem = session.images.find((item) => item.id === session.activeImageId) ?? null;
    if (activeItem) {
      void currentWindow.setTitle(localeText("main", "ui.ViewerWindow.b94b74c6ba16e123", {"value1": (activeItem.name)}));
      return;
    }

    void currentWindow.setTitle(localeText("main", "ui.ViewerWindow.0e6a9de70cbb4544"));
  }, [currentWindow, session]);

  const activeItem = useMemo(() => {
    return session.images.find((item) => item.id === session.activeImageId) ?? null;
  }, [session]);

  const handleOpenImage = (itemId: string) => {
    const navigationRevision = ++navigationRevisionRef.current;
    void invoke('set_viewer_active', { itemId, navigationRevision }).catch(error => {
      void currentWindow.setTitle(localeText("main", "ui.ViewerWindow.f6a8ae02a5244064", {"value1": (String(error))}));
    });
    setSession((current) => {
      if (!current.images.some((candidate) => candidate.id === itemId)) return current;
      return { ...current, activeImageId: itemId };
    });
  };

  const handleImageDeleted = (deletedItemId: string, deletedPath: string, nextItemId: string | null) => {
    setSession((current) => {
      const images = current.images.filter((candidate) => candidate.id !== deletedItemId && candidate.path !== deletedPath);
      const nextActiveImageId = nextItemId && images.some((candidate) => candidate.id === nextItemId)
        ? nextItemId
        : images[0]?.id ?? '';

      return {
        ...current,
        activeImageId: nextActiveImageId,
        images,
      };
    });

  };

  const handleToggleFavorite = async (itemId: string) => {
    const image = session.images.find((candidate) => candidate.id === itemId);
    if (!image) return;

    const result = await collectionService.setImageFavorite({
      imageId: image.indexedImageId ?? null,
      path: image.path,
      isFavorite: !image.isFavorite,
    });

    setSession((current) => updateSessionFavoriteState(current, result));
    void broadcastImageFavoriteUpdated(result);
  };

  const handleTopBarMouseDown = () => {
    void currentWindow.startDragging().catch(() => undefined);
  };

  const handleWindowModeChange = (mode: SmartImageViewerWindowMode) => {
    if (mode === 'minimized') {
      void currentWindow.minimize().catch(() => undefined);
      return;
    }

    void currentWindow.toggleMaximize().then(() => currentWindow.isMaximized())
      .then(maximized => setWindowMode(maximized ? 'full' : 'compact'));
  };

  const handleClose = () => {
    void currentWindow.close().catch(() => undefined);
  };

  if (!activeItem) {
    return (
      <div className="smart-image-viewer-window">
        <section className="smart-image-viewer smart-image-viewer--full" aria-label={localeText("main", "ui.ViewerWindow.f30bf149ae6cb490")}>
          <header className="smart-image-viewer__topbar" onMouseDown={handleTopBarMouseDown}>
            <div className="smart-image-viewer__topbar-left" />
            <strong className="smart-image-viewer__file-name">{localeText("main", "ui.ViewerWindow.c51f1bd023f5fc4a")}</strong>
            <div className="smart-image-viewer__window-actions">
              <Hint message={{id:"common.minimize"}}><button type="button" title={localeText("main", "ui.ViewerWindow.426c9c060aed8af3")} onClick={() => void currentWindow.minimize().catch(() => undefined)}><Icon name="minimize"/></button></Hint>
              <Hint message={{id:windowMode==='compact'?'common.maximize':'common.restore'}}><button type="button" title={localeText("main", "ui.ViewerWindow.79f3fb5facd1d34e")} onClick={() => void currentWindow.toggleMaximize().catch(() => undefined)}><Icon name="maximize"/></button></Hint>
              <Hint message={{id:"viewer.close"}}><button type="button" title={localeText("main", "ui.ViewerWindow.81b5f09076460974")} onClick={handleClose}><Icon name="close"/></button></Hint>
            </div>
          </header>
          <div className="smart-image-viewer__body">
            <main className="smart-image-viewer__stage">
              <div className="smart-image-viewer__fallback">
                <strong>{localeText("main", "ui.ViewerWindow.314499697e69da39")}</strong>
                <span>{localeText("main", "ui.ViewerWindow.ec637deb3c9cce17")}</span>
              </div>
            </main>
          </div>
          <footer className="smart-image-viewer__bottombar">
            <div className="smart-image-viewer__bottom-left">
              <span>{localeText("main", "ui.ViewerWindow.985fe3af959e8000")}</span>
            </div>
          </footer>
        </section>
      </div>
    );
  }

  return (
    <div className="smart-image-viewer-window">
      <SmartImageViewer
        item={activeItem}
        allImages={session.images}
        windowMode={windowMode}
        detailsOpen={detailsOpen}
        fileActions={tauriSmartImageViewerFileActions}
        onClose={handleClose}
        onWindowModeChange={handleWindowModeChange}
        onToggleDetails={() => setDetailsOpen((value) => !value)}
        onToggleFavorite={handleToggleFavorite}
        onOpenImage={handleOpenImage}
        onImageDeleted={handleImageDeleted}
        onTopBarMouseDown={handleTopBarMouseDown}
        onFullscreenChange={changeFullscreen}
      />
    </div>
  );
}

function updateSessionFavoriteState(session: ViewerSessionPayload, result: { imageId: string; path: string; isFavorite: boolean }): ViewerSessionPayload {
  const images = session.images.map((item) => {
    const matchesImageId = item.indexedImageId && item.indexedImageId === result.imageId;
    const matchesPath = item.path === result.path;
    if (!matchesImageId && !matchesPath) return item;
    return { ...item, isFavorite: result.isFavorite };
  });

  return { ...session, images };
}
