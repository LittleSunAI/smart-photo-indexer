import {text as localeText} from "../../shared/i18n/Runtime";
import {convertFileSrc} from "@tauri-apps/api/core";
import {invoke} from "../../shared/tauri/invoke";
import { isTauriRuntime } from '../../shared/tauri/runtime';

export interface ThumbnailRequest {
  imagePath: string;
  width: number;
  height: number;
  originalPreviewUrl?: string;
  signal?: AbortSignal;
  priority?: boolean;
  forceRebuild?: boolean;
  revision?: string | number;
}

export interface ThumbnailResult {
  sourcePath: string;
  thumbnailPath: string;
  thumbnailUrl: string;
  width: number;
  height: number;
  wasCreated: boolean;
  createdAtUnix: number;
  source: 'disk-cache' | 'original-fallback';
}

interface ThumbnailResultDto {
  sourcePath: string;
  thumbnailPath: string;
  width: number;
  height: number;
  wasCreated: boolean;
  createdAtUnix: number;
}

export interface ThumbnailService {
  getOrCreateThumbnail(request: ThumbnailRequest): Promise<ThumbnailResult>;
}

function clampDimension(value: number, fallback: number): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.max(64, Math.min(1024, Math.round(value)));
}

type QueueTask<T> = {
  signal?: AbortSignal;
  priority?: boolean;
  run(): Promise<T>;
  resolve(value: T): void;
  reject(error: unknown): void;
};

class ThumbnailQueue<T> {
  private activeCount = 0;
  private readonly tasks: QueueTask<T>[] = [];
  constructor(private readonly maxActive:number){}

  enqueue(task: QueueTask<T>): void {
    if (task.signal?.aborted) { task.reject(new DOMException(localeText("main", "ui.ThumbnailService.3bcc07c783e41487"), 'AbortError')); return; }
    if (this.tasks.length >= 128) { task.reject(new Error(localeText("main", "ui.ThumbnailService.d8da3ad8a4c251ec"))); return; }
    if (task.priority) this.tasks.unshift(task); else this.tasks.push(task);
    task.signal?.addEventListener('abort', () => {
      const index = this.tasks.indexOf(task);
      if (index >= 0) { this.tasks.splice(index, 1); task.reject(new DOMException(localeText("main", "ui.ThumbnailService.3bcc07c783e41487"), 'AbortError')); }
    }, { once: true });
    this.drain();
  }

  private drain(): void {
    while (this.activeCount < this.maxActive && this.tasks.length > 0) {
      const task = this.tasks.shift();
      if (!task) return;

      this.activeCount += 1;
      void task.run()
        .then(task.resolve)
        .catch(task.reject)
        .finally(() => {
          this.activeCount = Math.max(0, this.activeCount - 1);
          this.drain();
        });
    }
  }
}

const thumbnailQueue = new ThumbnailQueue<ThumbnailResultDto>(3);
const cacheQueue = new ThumbnailQueue<ThumbnailResultDto|null>(6);
function queued<T>(queue:ThumbnailQueue<T>,run:()=>Promise<T>,signal:AbortSignal,priority?:boolean){
  return new Promise<T>((resolve,reject)=>queue.enqueue({run,signal,priority,resolve,reject}));
}

interface SharedThumbnail {
  controller:AbortController;
  promise:Promise<ThumbnailResult>;
  subscribers:number;
  settled:boolean;
}

function originalFallbackResult(imagePath: string, width: number, height: number, originalPreviewUrl?: string): ThumbnailResult {
  return {
    sourcePath: imagePath,
    thumbnailPath: imagePath,
    thumbnailUrl: originalPreviewUrl ?? convertFileSrc(imagePath),
    width,
    height,
    wasCreated: false,
    createdAtUnix: Math.floor(Date.now() / 1000),
    source: 'original-fallback',
  };
}

export class TauriThumbnailService implements ThumbnailService {
  private readonly pending=new Map<string,SharedThumbnail>();
  async getOrCreateThumbnail(request: ThumbnailRequest): Promise<ThumbnailResult> {
    const imagePath = request.imagePath.trim();
    if (!imagePath) {
      throw new Error(localeText("main", "ui.ThumbnailService.fe483e3142e21ccd"));
    }

    const width = clampDimension(request.width, 360);
    const height = clampDimension(request.height, 260);

    if (!isTauriRuntime()) {
      return originalFallbackResult(imagePath, width, height, request.originalPreviewUrl);
    }

    request.signal?.throwIfAborted();
    const key=JSON.stringify([imagePath,width,height,request.revision??null,request.forceRebuild??false]);
    let shared=this.pending.get(key);
    if(!shared || shared.controller.signal.aborted){
      const controller=new AbortController();
      const load=async():Promise<ThumbnailResult>=>{
        // Ready thumbnails bypass slow decoding jobs. Both lanes stay bounded.
        const cached=request.forceRebuild?null:await queued(cacheQueue,()=>invoke<ThumbnailResultDto|null>('get_cached_thumbnail',{imagePath,width,height}),controller.signal,request.priority);
        controller.signal.throwIfAborted();
        const result=cached??await queued(thumbnailQueue,()=>invoke<ThumbnailResultDto>('get_or_create_thumbnail',{
          imagePath,width,height,forceRebuild:request.forceRebuild??false,
        }),controller.signal,request.priority);
        return {sourcePath:result.sourcePath,thumbnailPath:result.thumbnailPath,thumbnailUrl:convertFileSrc(result.thumbnailPath),
          width:result.width,height:result.height,wasCreated:result.wasCreated,createdAtUnix:result.createdAtUnix,source:'disk-cache'};
      };
      shared={controller,promise:load(),subscribers:0,settled:false};
      const owned=shared;
      const finish=()=>{owned.settled=true;if(this.pending.get(key)===owned)this.pending.delete(key);};
      owned.promise=owned.promise.then(value=>{finish();return value;},error=>{finish();throw error;});
      this.pending.set(key,shared);
    }
    const owned=shared;
    owned.subscribers++;
    return new Promise<ThumbnailResult>((resolve,reject)=>{
      let finished=false;
      const release=()=>{
        if(finished)return false;
        finished=true;request.signal?.removeEventListener('abort',cancel);owned.subscribers--;
        if(!owned.subscribers&&!owned.settled)owned.controller.abort();
        return true;
      };
      const cancel=()=>{if(release())reject(new DOMException(localeText("main", "ui.ThumbnailService.f8ab301827cda02f"),'AbortError'));};
      request.signal?.addEventListener('abort',cancel,{once:true});
      owned.promise.then(value=>{if(release())resolve(value);},error=>{if(release())reject(error);});
    });
  }
}

export const thumbnailService: ThumbnailService = new TauriThumbnailService();
