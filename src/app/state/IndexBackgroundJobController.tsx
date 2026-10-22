import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import { useEffect, useRef, type Dispatch } from 'react';
import type { AppAction } from './appActions';
import { fileSystemService } from '../../modules/filesystem/FileSystemService';
import { indexService } from '../../modules/index/IndexService';
import type { ImageItem } from '../../modules/explorer/types';
import type { AppErrorContext, AppErrorIssue } from '../../modules/errors/AppError';

interface IndexRescanJobRequest {
  id: string;
  rootPath: string | null;
  includeSubfolders: boolean;
  completion: Array<{ resolve(): void; reject(error: unknown): void }>;
}

interface RequestIndexRescanJobInput {
  rootPath: string | null;
  includeSubfolders: boolean;
}

type IndexRescanJobListener = (job: IndexRescanJobRequest) => void;

const indexRescanJobListeners = new Set<IndexRescanJobListener>();

export function requestIndexRescanJob(input: RequestIndexRescanJobInput): Promise<void> {
  return new Promise((resolve, reject) => {
    if (!indexRescanJobListeners.size) { reject(new Error(localeText("main", "ui.IndexBackgroundJobController.3ddb3ce543a4630f"))); return; }
    const job: IndexRescanJobRequest = {
      id: `index-rescan-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      rootPath: input.rootPath,
      includeSubfolders: input.includeSubfolders,
      completion: [{ resolve, reject }],
    };
    indexRescanJobListeners.forEach((listener) => listener(job));
  });
}

interface IndexBackgroundJobControllerProps {
  dispatch: Dispatch<AppAction>;
  reportAppError(error: unknown, context: AppErrorContext): AppErrorIssue;
}

export function IndexBackgroundJobController({ dispatch, reportAppError }: IndexBackgroundJobControllerProps) {
  useCurrentLocale();
  const queueRef = useRef<IndexRescanJobRequest[]>([]);
  const isRunningRef = useRef(false);
  const isMountedRef = useRef(false);
  const dispatchRef = useRef(dispatch);
  const reportAppErrorRef = useRef(reportAppError);
  const activeAbortRef = useRef<AbortController|null>(null);
  const activeJobRef = useRef<IndexRescanJobRequest|null>(null);
  dispatchRef.current = dispatch;
  reportAppErrorRef.current = reportAppError;

  useEffect(() => {
    isMountedRef.current = true;

    const runNextJob = () => {
      if (!isMountedRef.current || isRunningRef.current) return;

      const job = queueRef.current.shift();
      if (!job) return;

      isRunningRef.current = true;
      activeJobRef.current = job;
      const controller=new AbortController();activeAbortRef.current=controller;
      void processIndexRescanJob(job, action=>{if(isMountedRef.current)dispatchRef.current(action);}, reportAppErrorRef.current, controller.signal)
        .then(() => job.completion.forEach(waiter => waiter.resolve()), error => job.completion.forEach(waiter => waiter.reject(error)))
        .finally(() => {
          if(activeJobRef.current===job)activeJobRef.current=null;
          if(activeAbortRef.current===controller)activeAbortRef.current=null;
          isRunningRef.current = false;
          runNextJob();
        });
    };

    const handleJobRequest = (job: IndexRescanJobRequest) => {
      const same = [activeJobRef.current, ...queueRef.current].find(queued => queued?.rootPath === job.rootPath && queued.includeSubfolders === job.includeSubfolders);
      if(same){same.completion.push(...job.completion);return;}
      queueRef.current.push(job);
      runNextJob();
    };

    indexRescanJobListeners.add(handleJobRequest);
    return () => {
      isMountedRef.current = false;
      activeAbortRef.current?.abort();
      indexRescanJobListeners.delete(handleJobRequest);
      queueRef.current.forEach(job => job.completion.forEach(waiter => waiter.reject(new DOMException(localeText("main", "ui.IndexBackgroundJobController.4c3f9408977e17ed"), 'AbortError'))));
      queueRef.current = [];
    };
  }, []);

  return null;
}

async function processIndexRescanJob(
  job: IndexRescanJobRequest,
  dispatch: Dispatch<AppAction>,
  reportAppError: (error: unknown, context: AppErrorContext) => AppErrorIssue,
  signal: AbortSignal,
): Promise<void> {
  try {
    if (job.rootPath) {
      await fileSystemService.scanDirectory({
        path: job.rootPath,
        includeSubfolders: job.includeSubfolders,
        imagesOnly: true,
        signal,
        collectItems:false,
        onPage: async page=>{
          signal.throwIfAborted();
          await syncIndexForScannedItems(page.items.filter((item):item is ImageItem=>item.kind==='image'),dispatch);
        },
      });
    }

    signal.throwIfAborted();
    const rescanResult = await indexService.rescanIndexPaths({ rootPath: job.rootPath, signal });
    signal.throwIfAborted();
    dispatch({ type: 'index/rescanSucceeded', result: rescanResult });
  } catch (error) {
    if(signal.aborted)throw new DOMException(localeText("main", "ui.IndexBackgroundJobController.16e09d4b8a8122e1"), 'AbortError');
    const issue = reportAppError(error, { domain: 'sqlite', operation: localeText("main", "ui.IndexBackgroundJobController.10de801b50caed72"), relatedPath: job.rootPath });
    dispatch({ type: 'index/rescanFailed', errorMessage: issue.userMessage });
    throw error;
  }
}

async function syncIndexForScannedItems(images: ImageItem[], dispatch: Dispatch<AppAction>): Promise<void> {
  dispatch({ type: 'index/syncStarted', imageCount: images.length });

  try {
    const syncResult = await indexService.syncScannedImages(images);
    dispatch({ type: 'index/syncSucceeded', result: syncResult });
  } catch (error) {
    dispatch({ type: 'index/syncFailed', errorMessage: error instanceof Error ? error.message : String(error) });
    throw error;
  }
}
