import type {WorkingOrientation} from './EditorOrientation';
import {invoke} from "../../shared/tauri/invoke";
import type {EditorDraft,EditorDraftAck,EditorSession,EditorJob} from './EditorContracts';
import type {LocalOperation} from './EditorLocalDialog';
import type {MaskStroke} from './EditorAnnotations';
export const editorService={
  open:(sourcePath:string)=>invoke<string>('open_photo_editor',{sourcePath}),
  session:()=>invoke<EditorSession>('get_editor_session'),
  save:(epoch:string,expectedRevision:number,draft:EditorDraft)=>invoke<EditorDraftAck>('save_editor_draft',{epoch,expectedRevision,draft}),
  navigate:(epoch:string,revision:number,action:'return'|'archive'|'close',discardUnsaved=false)=>invoke<void>('finish_editor_navigation',{epoch,revision,action,discardUnsaved}),
  local:(epoch:string,expectedRevision:number,draft:EditorDraft,operation:LocalOperation)=>invoke<Pick<EditorSession,'project'|'assets'|'steps'>>('apply_editor_local_operation',{epoch,expectedRevision,draft,operation}),
  mask:(epoch:string,expectedRevision:number,draft:EditorDraft,annotationId:string,strokes:MaskStroke[])=>invoke<Pick<EditorSession,'project'|'assets'|'steps'>>('save_editor_mask',{epoch,expectedRevision,draft,annotationId,strokes}),
  library:(epoch:string,action:string,expectedRevision?:number,id?:string,text?:string,discardChanges=false)=>invoke<Pick<EditorSession,'project'|'assets'|'steps'>>('editor_library_command',{epoch,action,expectedRevision,id,text,discardChanges}),
  import:(epoch:string,path?:string)=>invoke<Pick<EditorSession,'project'|'assets'|'steps'>|null>('import_editor_file',{epoch,path}),
  importUrl:(epoch:string,url:string)=>invoke<Pick<EditorSession,'project'|'assets'|'steps'>>('import_editor_url',{epoch,url}),
  export:(epoch:string,assetId:string,orientation:WorkingOrientation,format:string,quality:number,target?:string,useDefault=false)=>invoke<{path:string;sha256:string}|null>('export_editor_image',{epoch,assetId,workingTurn:orientation.rotation,mirrored:orientation.mirrored,format,quality,target,useDefault}),
  replacementPreview:(epoch:string,assetId:string,orientation:WorkingOrientation,quality:number)=>invoke<{id:string;path:string;fileName:string;format:string;width:number;height:number}>('preview_editor_replacement',{epoch,assetId,workingTurn:orientation.rotation,mirrored:orientation.mirrored,quality}),
  replaceOriginal:(epoch:string,id:string)=>invoke<{path:string;state:string}>('replace_editor_original',{epoch,id,confirmed:true}),
  publish:(epoch:string,id:string,assetId:string,orientation:WorkingOrientation,format:string,quality:number,target?:string)=>invoke<{path:string;state:string}|null>('publish_editor_copy',{epoch,id,assetId,workingTurn:orientation.rotation,mirrored:orientation.mirrored,format,quality,target}),
  package:(epoch:string,importPackage:boolean,path?:string)=>invoke<{projectId?:string;path?:string}|null>('editor_project_package',{epoch,import:importPackage,path}),
  capacity:(epoch:string,draft:EditorDraft,annotationId?:string)=>invoke<{used:number;max:number;remaining:number}>('editor_reference_capacity',{epoch,draft,annotationId}),
  attach:(epoch:string,expectedRevision:number,annotationId:string,replace:boolean,paths?:string[],assetIds?:string[])=>invoke<Pick<EditorSession,'project'|'assets'|'steps'>|null>('attach_editor_references',{epoch,expectedRevision,annotationId,replace,paths,assetIds}),
  attachStep:(epoch:string,expectedRevision:number,annotationId:string,replace:boolean,stepId:string)=>invoke<Pick<EditorSession,'project'|'assets'|'steps'>>('attach_editor_references',{epoch,expectedRevision,annotationId,replace,stepId}),
  checkpoint:(epoch:string,revision:number)=>invoke<import('./EditorContracts').ProjectLifecycle>('save_editor_checkpoint',{epoch,revision}),
  lifecycle:(epoch:string)=>invoke<import('./EditorContracts').ProjectLifecycle>('get_editor_lifecycle',{epoch}),
  preview:(epoch:string,draft:EditorDraft)=>invoke<unknown>('preview_editor_request',{epoch,draft}),
  start:(epoch:string,id:string,expectedRevision:number,draft:EditorDraft,credentialRef:string,consent:boolean,budgetMicros:number,concurrency:number,sourceStepId?:string,expectedPlanHash?:string)=>invoke<{id:string}>('start_editor_job',{epoch,id,expectedRevision,draft,credentialRef,consent,budgetMicros,concurrency,sourceStepId,expectedPlanHash}),
  jobs:(epoch?:string)=>invoke<EditorJob[]>('get_editor_jobs',{epoch}),
  jobCommand:(id:string,action:string,epoch?:string)=>invoke<EditorJob[]>('editor_job_command',{epoch,id,action}),
  reconcileRequest:(id:string,requestId:string,confirmed:boolean,epoch?:string)=>invoke<EditorJob[]>('reconcile_editor_request',{epoch,id,requestId,confirmed}),
};
