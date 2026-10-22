import {text as localeText} from "../../shared/i18n/Runtime";
import {loadUiPreferences,saveUiPreferences,manualQueueTargetVersion} from '../../shared/UiPreferences';
import {invoke} from "../../shared/tauri/invoke";
import type { QueueTask, TaskQueueRunConfig } from './types';

export interface NativeTask {
  config?:TaskQueueRunConfig;
  workVisible?:boolean;
  runRevision?:number;
  id: string; imageId: string; sourceItemId: string | null; path: string; status: string;
  currentStartedAtUnix?:number|null;
  ordinal?:number;
    completedPasses: number; error: string | null; metadataCommitted: boolean; renameStatus: string;
}
export interface NativeJob {
  total?:number;done?:number;failed?:number;active?:number;
  workVisible?:boolean;
  queueMode?:'direct'|'manual';
  mixedContexts?:boolean;
  notice?:string;
  estimatedPendingUsd?:number;
  jobId: string; status: string; config: TaskQueueRunConfig; error: string | null;
  revision: number; createdAtUnix: number; spentUsd?: number; reservedUsd?: number; unknownCosts?: number; unresolvedRequests?: number;
  counters?: { total: number; pending: number; running: number; done: number; failed: number; cancelled: number; retryWaiting: number };
}
export interface ProcessingSnapshot {
  queueMode?:'direct'|'manual'|null;
  aggregate?:QueueAggregate;
  jobs: NativeJob[]; workingJobs?:NativeJob[]; job: NativeJob | null; tasks: NativeTask[]; revision: number; hasMore: boolean;
}
export interface QueueAggregate{active:number;ready:number;stoppable:number;runActive:boolean;preflight:boolean;otherActive?:number;counters?:import('./types').TaskQueueCounters}
export interface PreparedQueue {jobId:string;queueMode?:'direct'|'manual';status?:string;duplicate?:boolean;totalCount:number;readyCount:number;duplicateCount:number;skippedCount:number;alreadyReady?:boolean;message?:string}
const preparations = new Map<string, {target:string|null;mode:string;promise:Promise<PreparedQueue>}>();
export const nativeQueueService = {
  async createEmpty(requestKey:string):Promise<{jobId:string;preferenceError?:string}>{const version=manualQueueTargetVersion();const result=await invoke<{jobId:string;preferenceError?:string}>('create_processing_queue',{requestKey});if(version===manualQueueTargetVersion()){try{saveUiPreferences({queueManualTarget:result.jobId});}catch(error){result.preferenceError=String(error);}}window.dispatchEvent(new CustomEvent('smart-photo-indexer:select-processing-job',{detail:result.jobId}));window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));return result;},
  contexts(jobId?:string,retry=false,queueMode?:'direct'|'manual'):Promise<Array<{providerMode:string;model:string;total:number}>>{return invoke('get_processing_queue_contexts',{jobId,retry,queueMode});},
  async commandAll(action:string,queueMode:'direct'|'manual'):Promise<{errors?:Array<{jobId:string;error:string}>;cancelled?:boolean}>{const result=await invoke('processing_queue_command',{action,queueMode});window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));return result as {errors?:Array<{jobId:string;error:string}>;cancelled?:boolean};},
  async remove(scope:'photo'|'job'|'allWorking',jobId?:string,taskId?:string,queueMode?:'direct'|'manual'):Promise<{removedCount:number;preferenceError?:string}>{const result=await invoke<{removedCount:number;preferenceError?:string}>('remove_processing_work',{scope,jobId,taskId,queueMode});const target=loadUiPreferences().queueManualTarget;if(scope==='allWorking'&&queueMode!=='direct'||scope==='job'&&jobId===target){try{saveUiPreferences({queueManualTarget:null});}catch(error){result.preferenceError=String(error);}}window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));return result;},
  prepare(tasks: QueueTask[], config: TaskQueueRunConfig,targetJobId?:string|null): Promise<PreparedQueue> {
    const key=tasks[0]?.id;
    if (!key) return Promise.reject(new Error(localeText("main", "ui.NativeQueueService.5079e49dd219cd18")));
    const mode=config.queueMode??(config.autoStart?'direct':'manual'),target=targetJobId??null;
    const existing=preparations.get(key);if(existing){if(existing.target!==target||existing.mode!==mode)return Promise.reject(new Error(localeText("main", "ui.NativeQueueService.17bbf4a9efeca665")));return existing.promise;}
    const promise=invoke<PreparedQueue>('prepare_processing_job',{input:{ requestKey:key,targetJobId:targetJobId??null,queueMode:config.queueMode??(config.autoStart?'direct':'manual'),config,credentialRef:config.credentialRef,tasks:tasks.map(task=>({targetPath:task.targetPath,sourceItemId:task.sourceItemId,renamePlanId:task.renamePlanId})) }});
    preparations.set(key,{promise,target,mode});void promise.finally(()=>preparations.delete(key)).catch(()=>{});
    return promise;
  },
  cancelPreparation(requestKey:string):Promise<void>{return invoke('cancel_processing_preparation',{requestKey});},
  snapshot(jobId?:string,afterRevision=0,hideFinished=false,queueMode?:'direct'|'manual'):Promise<ProcessingSnapshot>{return invoke('get_processing_snapshot',{jobId,afterRevision,hideFinished,queueMode});},
  async command(jobId:string,action:string,requestKey?:string):Promise<void>{await invoke('processing_job_command',{jobId,action,requestKey});window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));},
  async setConcurrency(jobId:string,requested:number):Promise<void>{await invoke('set_processing_concurrency',{jobId,requested});window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));},
  async reconcile(jobId:string):Promise<{ checked:number; resolved:number; errors:string[] }>{const result=await invoke<{ checked:number; resolved:number; errors:string[] }>('reconcile_processing_attempts',{jobId});window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));return result;},
};
