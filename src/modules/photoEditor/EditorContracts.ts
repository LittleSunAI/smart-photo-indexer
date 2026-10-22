import {text as localeText} from "../../shared/i18n/Runtime";
import type {WorkingOrientation} from './EditorOrientation';
import type {EditHistory} from './EditorEditHistory';
/** Versioned native/editor boundary. Paths are native-owned; no provider secret in DTOs. */
export const EDITOR_CONTRACT_VERSION=3 as const;
export const editorOperations=[
  {id:'universal-edit',get label() { return localeText("editor", "ui.EditorContracts.249bb780afe82272"); },get help() { return localeText("editor", "ui.EditorContracts.08b0d3258e0a33b9"); }},
  {id:'replace-object',get label() { return localeText("editor", "ui.EditorContracts.33f2d433ba1349d3"); },get help() { return localeText("editor", "ui.EditorContracts.3c6456af5b57b063"); }},
  {id:'remove-object',get label() { return localeText("editor", "ui.EditorContracts.18217ab9b99d98e6"); },get help() { return localeText("editor", "ui.EditorContracts.5130ba7552ad6711"); }},
  {id:'remove-background',get label() { return localeText("editor", "ui.EditorContracts.7668a473b6698d06"); },get help() { return localeText("editor", "ui.EditorContracts.37c15a33d0f17fe1"); }},
  {id:'replace-background',get label() { return localeText("editor", "ui.EditorContracts.fb0c61c3b96d9156"); },get help() { return localeText("editor", "ui.EditorContracts.42d11b4f3a396180"); }},
  {id:'outpaint',get label() { return localeText("editor", "ui.EditorContracts.f0865121174c419e"); },get help() { return localeText("editor", "ui.EditorContracts.bbce5bcbfe5a0775"); }},
  {id:'upscale',get label() { return localeText("editor", "ui.EditorContracts.5406f938afdbcb02"); },get help() { return localeText("editor", "ui.EditorContracts.22420dfbcb7f5110"); }},
  {id:'relight',get label() { return localeText("editor", "ui.EditorContracts.ba7d12ae1c485ae4"); },get help() { return localeText("editor", "ui.EditorContracts.1105327482a3268c"); }},
  {id:'recolor',get label() { return localeText("editor", "ui.EditorContracts.b6fe264f57e22a34"); },get help() { return localeText("editor", "ui.EditorContracts.7c6ca788a8322bc2"); }},
  {id:'compose',get label() { return localeText("editor", "ui.EditorContracts.ba23c4878a71d0b2"); },get help() { return localeText("editor", "ui.EditorContracts.5482f419e79f1e29"); }},
  {id:'restore',get label() { return localeText("editor", "ui.EditorContracts.33b266af58baab0e"); },get help() { return localeText("editor", "ui.EditorContracts.8c44a66f463ae0d8"); }},
] as const;
export type EditorOperationId=typeof editorOperations[number]['id'];
export type EditorTool='select'|'pan'|'point'|'rect'|'mask'|'eraser';
export type EditorProfile='economy'|'balanced'|'quality';
export type QuarterTurn=0|1|2|3;
export interface NormalizedPoint {x:number;y:number}
export interface AnnotationBase {id:string;assetId:string;instruction:string;visible:boolean;included:boolean}
export type EditorAnnotation=
  |(AnnotationBase&{kind:'point';point:NormalizedPoint})
  |(AnnotationBase&{kind:'rect';start:NormalizedPoint;end:NormalizedPoint})
  |(AnnotationBase&{kind:'mask';maskAssetId:string;version:number});
export interface EditorDraft {
  version:3;assetId:string;orientation:WorkingOrientation;editHistory:EditHistory;tool:EditorTool;
  operation:EditorOperationId;profile:EditorProfile;modelId:string|null;prompt:string;
  formValues:Record<string,unknown>;annotations:EditorAnnotation[];selectedAnnotationId:string|null;
  annotationReferences:Record<string,Array<{assetId:string;included:boolean;displayName?:string;roleDescription?:string}>>;selectedMaskIds:string[];references:Array<{assetId:string;role:string}>;
  viewport:{zoom:number;panX:number;panY:number};
  layout:{leftWidth:number;rightWidth:number;historyHeight:number;rightTab:'edit'|'tasks'|'sources'};
}
export interface EditorAsset {
  id:string;kind:'image'|'mask';relativePath:string;previewPath:string;
  sha256:string;width:number;height:number;bytes:number;contentType:string;
}
export interface EditorStep {
  id:string;projectId:string;assetId:string;parentIds:string[];
  hidden?:boolean;
  operation:'source'|'import'|'crop'|'resize'|'flip'|EditorOperationId;
  title:string;createdAtUnix:number;jobId:string|null;
  parameters:Record<string,unknown>;annotations:EditorAnnotation[];workingOrientation:WorkingOrientation;
}
export interface ProjectLifecycle{version:1;projectId:string;contentGeneration:number;savedContentGeneration:number|null;modifiedAtUnix:number;checkpointId:string|null;savedAtUnix:number|null;dirty:boolean;retained:boolean}
export interface EditorProject {
  lifecycle?:ProjectLifecycle;
  id:string;title:string;sourceName:string;sourceSha256:string;
  activeStepId:string;rootStepId:string;revision:number;draft:EditorDraft;
  createdAtUnix:number;updatedAtUnix:number;
}
export interface EditorSession {
  version:3;label:string;epoch:string;originKind:'archive'|'viewer';requestedStepId?:string|null;
  project:EditorProject;assets:EditorAsset[];steps:EditorStep[];
}
export type EditorJobState='validating'|'awaitingConsent'|'reserved'|'preparingInputs'|'ready'|'submitting'|'queued'|'running'|'downloading'|'committingResult'|'completed'|'failed'|'cancelRequested'|'cancelled'|'interrupted'|'unknownSubmit';
export interface EditorJob {
  readOnly?:boolean;
  results?:Array<{stepId:string;assetId:string;outputIndex:number;hidden:boolean}>;
  id:string;projectId:string;state:EditorJobState;operation:EditorOperationId;
  modelId:string;requestId:string|null;sourceAssetId:string;
  reservedMicros:number;spentMicros:number|null;error:string|null;admission?:{kind:'restorePending'|'unknownRequests'|'slotsBusy';provider:'fal';details:Record<string,number>}|null;
  resultAssetIds:string[];createdAtUnix:number;updatedAtUnix:number;requestedOutputCount?:number;providerOutputCount?:number|null;
  startedAtUnix?:number|null;completedAtUnix?:number|null;
  sourceSize?:{width:number;height:number};requestedSize?:unknown;requestedResolution?:string|null;requestedQuality?:string|null;resultSizes?:Array<{width:number;height:number}>;
}
export type PublicationState='planned'|'fileSaved'|'indexPending'|'completed';
export interface EditorDraftAck {lifecycle?:ProjectLifecycle;projectId:string;revision:number;savedAtUnix:number}

export function initialEditorDraft(assetId:string):EditorDraft{
  return {version:3,assetId,orientation:{rotation:0,mirrored:false},editHistory:{undo:[],redo:[]},tool:'select',operation:'universal-edit',profile:'economy',modelId:null,prompt:'',formValues:{},annotations:[],selectedAnnotationId:null,selectedMaskIds:[],references:[],annotationReferences:{},viewport:{zoom:1,panX:0,panY:0},layout:{leftWidth:184,rightWidth:316,historyHeight:116,rightTab:'edit'}};
}
