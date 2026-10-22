import type {EditorDraft} from './EditorContracts';
import {instructionRequest} from './EditorInstructionState';
import {orientPoint} from './EditorOrientation';
function canonical(value:unknown):unknown {if(Array.isArray(value))return value.map(canonical);if(value&&typeof value==='object')return Object.fromEntries(Object.entries(value).sort(([a],[b])=>a.localeCompare(b)).map(([key,value])=>[key,canonical(value)]));return value;}
/** Changes to navigation, visibility and tool affordances must never trigger another price request. */
export function requestIdentity(d:EditorDraft):string {
 const ignored=new Set(['__contexts','__pricingCredential','__pricingRevision','__brushDiameter','__consentScope','__budgetUsd','__overallBudgetUsd','__budgetSince','__lastChoices','__concurrency','__pricingCredential','__pricingRevision','__rawJSON','__jsonParameterBase','__jsonOutputSizeBase','__instructionContext','__customProfile']);
 return JSON.stringify(canonical({spatialCompilerVersion:1,assetId:d.assetId,orientation:d.orientation,operation:d.operation,profile:d.profile,modelId:d.modelId,instructions:instructionRequest(d),references:d.references,annotationReferences:d.annotationReferences,selectedMaskIds:[...d.selectedMaskIds].sort(),annotations:d.annotations.filter(a=>a.kind==='mask'&&a.included&&d.selectedMaskIds.includes(a.id)).map(({visible,...a})=>a),formValues:Object.fromEntries(Object.entries(d.formValues).filter(([k])=>!ignored.has(k)))}));
}
/** Price capabilities do not depend on a point's position. Launch still uses full requestIdentity. */
export function priceIdentity(d:EditorDraft,coordinateTariff=false):string{
 const value=JSON.parse(requestIdentity(d)) as Record<string,unknown>;
 delete (value.formValues as Record<string,unknown>).__outpaintAutoCrop;
 const instruction=value.instructions as {globalText?:string;tasks?:Array<{target:string;text:string}>}|null;
 if(instruction){const texts=[instruction.globalText??'',...(instruction.tasks??[]).map(t=>t.text)];
  const coordinateCharacters=coordinateTariff?(instruction.tasks??[]).reduce((count,task)=>{
   const annotation=d.annotations.find(a=>a.id===task.target);
   const points=annotation?.kind==='point'?[annotation.point]:annotation?.kind==='rect'?[annotation.start,annotation.end]:[];
   return count+points.reduce((n,point)=>{const p=orientPoint(point,d.orientation);return n+(p.x*100).toFixed(1).length+(p.y*100).toFixed(1).length;},0);
  },0):undefined;
  value.instructions={characters:coordinateTariff?texts.reduce((n,t)=>n+Array.from(t).length,0):undefined,hasText:texts.some(t=>t.trim().length>0),coordinateCharacters,targets:instruction.tasks?.map(t=>t.target),hasNonAscii:d.modelId==='fal-ai/bria/expand'&&texts.some(t=>/[^\x00-\x7F]/.test(t))};}
 return JSON.stringify(canonical(value));
}
export function commonFormValues(values:EditorDraft['formValues']):EditorDraft['formValues'] {
 return Object.fromEntries(Object.entries(values).filter(([key])=>['__outputSizeMode','__pricingCredential','__pricingRevision','__brushDiameter','__consentScope','__budgetUsd','__overallBudgetUsd','__budgetSince','__lastChoices','__concurrency','__preserveColor','__instructionContext','__customProfile','__maskStrokes','__maskAnnotationId'].includes(key)||key.startsWith('__outpaint')));
}
export interface LaunchIntent {
 id:string;epoch:string;projectId:string;sourceStepId:string;revision:number;draft:EditorDraft;plan:unknown;
 signature:string;credentialRef:string;budgetMicros:number;concurrency:number;
}
