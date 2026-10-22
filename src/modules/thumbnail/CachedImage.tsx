import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect, useRef, useState,type ReactNode } from 'react';
import { thumbnailService } from './ThumbnailService';
import { previewKey,rememberedPreview,rememberPreview,forgetPreview,forgetAllPreviews } from './RenderedPreviewCache';

export type CachedImageVariant = 'card' | 'details';

interface CachedImageProps {
  imagePath: string;
  originalPreviewUrl?: string;
  alt?: string;
  variant?: CachedImageVariant;
  className?: string;
  fallbackClassName?: string;
  fallbackLabel?: string;
  width?: number;
  height?: number;
  revision?: string | number;
  fallback?:ReactNode;
}

type ThumbnailLoadState = 'idle' | 'waiting' | 'loading' | 'loaded' | 'fallback' | 'failed';

export function CachedImage({
  imagePath,
  originalPreviewUrl,
  alt = '',
  variant = 'card',
  className,
  fallbackClassName = 'mock-photo mock-photo-birthday',
  fallbackLabel = 'image',
  width,
  height,
  revision,
  fallback,
}: CachedImageProps) {
  useCurrentLocale();
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [isInViewport, setIsInViewport] = useState(variant === 'details');
  const targetWidth=width ?? (variant==='details'?640:360),targetHeight=height ?? (variant==='details'?420:260);
  const sourceKey=previewKey(imagePath,targetWidth,targetHeight,revision);
  const loadedKey=useRef(sourceKey);
  const [thumbnailUrl, setThumbnailUrl] = useState<string | null>(()=>rememberedPreview(sourceKey));
  const [retryEpoch,setRetryEpoch]=useState(0);
  const imageFailures=useRef(0);
  const [loadState, setLoadState] = useState<ThumbnailLoadState>(variant === 'details' ? 'idle' : 'waiting');
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [cacheEpoch, setCacheEpoch] = useState(0);
  useEffect(() => {
    const cleared = () => {forgetAllPreviews();setCacheEpoch(value => value + 1);};
    window.addEventListener('smart-photo-indexer:thumbnail-cache-cleared', cleared);
    return () => window.removeEventListener('smart-photo-indexer:thumbnail-cache-cleared', cleared);
  }, []);

  useEffect(() => {
    if (variant === 'details') {
      setIsInViewport(true);
      return;
    }

    const root = rootRef.current;
    if (!root) return;

    if (!('IntersectionObserver' in window)) {
      setIsInViewport(true);
      return;
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setIsInViewport(true);
          observer.disconnect();
        }
      },
      { root: null, rootMargin: '700px 0px', threshold: 0.01 },
    );

    observer.observe(root);
    return () => observer.disconnect();
  }, [imagePath, variant]);

  useEffect(() => {
    let isCancelled = false;
    const abort = new AbortController();
    const targetWidth = width ?? (variant === 'details' ? 640 : 360);
    const targetHeight = height ?? (variant === 'details' ? 420 : 260);

    if(loadedKey.current!==sourceKey){loadedKey.current=sourceKey;imageFailures.current=0;setThumbnailUrl(rememberedPreview(sourceKey));}
    setErrorMessage(null);

    if (!isInViewport) {
      setLoadState('waiting');
      return () => {
        isCancelled = true;
      };
    }

    setLoadState('loading');
    const root=rootRef.current;
    const rect=root?.getBoundingClientRect(),grid=root?.closest('.explorer-grid')?.getBoundingClientRect();
    const visible=Boolean(rect&&rect.bottom>Math.max(0,grid?.top??0)&&rect.top<Math.min(window.innerHeight,grid?.bottom??window.innerHeight));

    thumbnailService
      .getOrCreateThumbnail({ imagePath, width: targetWidth, height: targetHeight, originalPreviewUrl, signal: abort.signal, priority: variant === 'details'||visible, forceRebuild:imageFailures.current>0,revision })
      .then((result) => {
        if (isCancelled) return;
        if (result.thumbnailUrl) {
          const url=imageFailures.current>0?`${result.thumbnailUrl}${result.thumbnailUrl.includes('?')?'&':'?'}repaired=${Date.now()}`:result.thumbnailUrl;
          rememberPreview(sourceKey,url);
          setThumbnailUrl(url);
          setLoadState('loaded');
          return;
        }
        setLoadState('fallback');
      })
      .catch((error) => {
        if (isCancelled) return;
        const message = error instanceof Error ? error.message : typeof error === 'string' ? error : localeText("main", "ui.CachedImage.05f48e3848537cef");
        setErrorMessage(message);
        setLoadState('failed');
      });

    return () => {
      isCancelled = true;
      abort.abort();
    };
  }, [height, imagePath, isInViewport, originalPreviewUrl, variant, width, revision, cacheEpoch,sourceKey,retryEpoch]);

  const handleImageError = () => {
    forgetPreview(sourceKey);
    setThumbnailUrl(null);
    if(imageFailures.current++===0){setRetryEpoch(value=>value+1);return;}
    setErrorMessage(localeText("main", "ui.CachedImage.8b70530b7ddad440"));
    setLoadState('failed');
  };

  const visibleUrl=loadedKey.current===sourceKey?thumbnailUrl:rememberedPreview(sourceKey);
  if (visibleUrl) {
    return (
      <div ref={rootRef} className={`${className ?? ''} cached-image cached-image--${variant} cached-image--${loadState}`.trim()}>
        <img src={visibleUrl} alt={alt} loading={variant === 'card' ? 'lazy' : 'eager'} draggable={false} onError={handleImageError} />
      </div>
    );
  }

  return (
    <div
      ref={rootRef}
      className={`${className ?? ''} cached-image cached-image--${variant} cached-image--placeholder cached-image--${loadState}`.trim()}
      aria-busy={loadState === 'loading'}
      title={errorMessage ?? undefined}
    >
      {fallback??<div className={fallbackClassName}>
        <span>{loadState === 'waiting' ? localeText("main", "ui.CachedImage.9fa4cf421c3f4f09") : loadState === 'loading' ? localeText("main", "ui.CachedImage.4029978d17c7638b") : loadState === 'failed' ? localeText("main", "ui.CachedImage.eeb19d286b095c1c") : fallbackLabel}</span>
      </div>}
      {loadState === 'failed' && !fallback ? <Hint message={{id:"common.retry-read",args:{раздел:localeText("main", "ui.CachedImage.b63385946742484d")}}}><button type="button" className="thumbnail-error-badge" title={errorMessage??undefined} aria-label={localeText("main", "ui.CachedImage.2e27d2be6925b6c6")} onClick={event=>{event.stopPropagation();imageFailures.current=1;setRetryEpoch(value=>value+1);}}>{localeText("main", "ui.CachedImage.d306d4f901475487")}</button></Hint> : null}
    </div>
  );
}
