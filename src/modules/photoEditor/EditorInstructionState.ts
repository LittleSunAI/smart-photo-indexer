import catalog from './catalog.json';
import type {EditorDraft,EditorOperationId} from './EditorContracts';
import {effectiveParameter} from './EditorEffectiveParameters';

export interface InstructionOperation {
  mode:'single'|'multi';target:string;texts:Record<string,string>;enabled:string[];globalText:string;reviewed:boolean;
}
export interface InstructionState {
  version:1;operations:Partial<Record<EditorOperationId,InstructionOperation>>;
  projection:{prompt:string;annotations:Record<string,string>};
}
export const localInstructions=(operation:string)=>['universal-edit','replace-object','relight','recolor'].includes(operation);
export function acceptsInstruction(d:EditorDraft):boolean {
  const route=catalog.routes.find(r=>r.id===d.modelId);
  if(route?.id==='topaz/upscale/image/generative'&&!String(effectiveParameter(d,'model','')).startsWith('Redefine'))return false;
  return Boolean(route?.inputSchema.properties&&'prompt' in route.inputSchema.properties);

}
const empty=():InstructionOperation=>({mode:'multi',target:'image',texts:{},enabled:[],globalText:'',reviewed:true});
const projection=(d:EditorDraft)=>({prompt:d.prompt,annotations:Object.fromEntries(d.annotations.map(a=>[a.id,a.instruction]))});
export function instructionState(d:EditorDraft):InstructionState {
  const stored=d.formValues.__instructionContext as InstructionState|undefined;
  if(stored?.version===1&&stored.operations&&stored.projection)return stored;
  const current=empty(),locals=d.annotations.filter(a=>a.instruction.trim());
  if(d.prompt)current.texts.image=d.prompt;
  for(const a of locals)current.texts[a.id]=a.instruction;
  current.enabled=locals.filter(a=>a.included).map(a=>a.id);
  if(locals.length){current.mode='multi';current.globalText=d.prompt;current.reviewed=false;if(localInstructions(d.operation)){const target=locals.find(a=>a.id===d.selectedAnnotationId&&['point','rect'].includes(a.kind))??locals.find(a=>['point','rect'].includes(a.kind));if(target)current.target=target.id;}}else if(d.prompt&&localInstructions(d.operation)&&d.selectedAnnotationId&&d.annotations.some(a=>a.id===d.selectedAnnotationId&&['point','rect'].includes(a.kind))){current.target=d.selectedAnnotationId;current.texts[current.target]=d.prompt;}
  // Old saved model contexts are retained for explicit review, never silently chosen.
  const operations:InstructionState['operations']={[d.operation]:current};
  const contexts=d.formValues.__contexts as Record<string,{prompt?:string}>|undefined;
  for(const [key,value] of Object.entries(contexts??{})) {
    const op=key.split('|')[0] as EditorOperationId;
    if(value.prompt&&op!==d.operation&&!operations[op])operations[op]={...empty(),texts:{image:value.prompt},reviewed:false};
  }
  return {version:1,operations,projection:projection(d)};
}
export const currentInstruction=(d:EditorDraft)=>instructionState(d).operations[d.operation]??empty();
function project(d:EditorDraft,state:InstructionState):EditorDraft {
  const stored=state.operations[d.operation]??empty();
  const op:InstructionOperation=localInstructions(d.operation)?{...stored,mode:'multi' as const,reviewed:true,enabled:d.annotations.filter(a=>a.included&&['point','rect'].includes(a.kind)).map(a=>a.id)}:{...stored,mode:'single' as const,reviewed:true,target:'image',texts:{...stored.texts,image:stored.texts.image??stored.globalText}};
  state={...state,operations:{...state.operations,[d.operation]:op}};
  const prompt=localInstructions(d.operation)?[...new Set([op.globalText,op.texts.image??''].filter(t=>t.trim()))].join('\n'):(op.texts.image??'');
  const annotations=d.annotations.map(a=>({...a,instruction:op.texts[a.id]??''}));
  const result={...d,prompt,annotations};
  return {...result,formValues:{...d.formValues,__instructionContext:{...state,projection:projection(result)}}};
}
export function hydrateInstructions(d:EditorDraft):EditorDraft {
  const state=instructionState(d),op=state.operations[d.operation]??empty();
  const external=state.projection.prompt!==d.prompt||d.annotations.some(a=>a.id in state.projection.annotations&&state.projection.annotations[a.id]!==a.instruction);
  if(external){const texts={...op.texts};if(d.prompt!==state.projection.prompt)texts.image=d.prompt;
    for(const a of d.annotations)if(a.instruction!==state.projection.annotations[a.id])texts[a.id]=a.instruction;
    return project(d,{...state,operations:{...state.operations,[d.operation]:{...op,texts,reviewed:false}}});}
  return project(d,state);
}
export function changeInstruction(d:EditorDraft,change:(op:InstructionOperation)=>InstructionOperation):EditorDraft {
  const state=instructionState(d);
  return project(d,{...state,operations:{...state.operations,[d.operation]:change(state.operations[d.operation]??empty())}});
}
export const setInstructionText=(d:EditorDraft,text:string,target=currentInstruction(d).target)=>changeInstruction(d,op=>({...op,texts:{...op.texts,[target]:text}}));
export const setInstructionTarget=(d:EditorDraft,target:string)=>changeInstruction({...d,selectedAnnotationId:target==='image'?null:d.annotations.some(a=>a.id===target&&a.kind!=='mask')?target:d.selectedAnnotationId,selectedMaskIds:target==='image'?[]:d.selectedMaskIds},op=>({...op,target}));
export function switchInstructionOperation(d:EditorDraft,operation:EditorOperationId):EditorDraft {
  const state=instructionState(d);return project({...d,operation},state);
}
export function reconcileInstructions(previous:EditorDraft,next:EditorDraft):EditorDraft {
  if(!next.formValues.__instructionContext)return hydrateInstructions(next);
  if(next.operation!==previous.operation)return project(next,instructionState(next));
  const created=next.annotations.find(a=>!previous.annotations.some(old=>old.id===a.id)&&['point','rect'].includes(a.kind));
  const selected=next.selectedAnnotationId!==previous.selectedAnnotationId?next.annotations.find(a=>a.id===next.selectedAnnotationId&&['point','rect'].includes(a.kind)):null;
  const op=currentInstruction(next);
  if(localInstructions(next.operation)&&(created||selected)){
    const target=(created??selected)!.id;
    return changeInstruction(next,value=>({...value,target}));
  }
  if(localInstructions(next.operation)&&op.target!=='image'&&!next.annotations.some(a=>a.id===op.target)){
    const selected=next.annotations.find(a=>a.id===next.selectedAnnotationId&&['point','rect'].includes(a.kind));
    return changeInstruction(next,value=>({...value,target:selected?.id??'image'}));
  }
  return project(next,instructionState(next));
}
export function instructionRequest(d:EditorDraft):unknown {
  if(!acceptsInstruction(d))return null;
  const op=currentInstruction(d),local=localInstructions(d.operation);
  const targets=local?d.annotations.filter(a=>a.included&&['point','rect'].includes(a.kind)&&op.texts[a.id]?.trim()).map(a=>a.id):['image'];
  return {mode:local?'multi':'single',reviewed:true,globalText:local?[...new Set([op.globalText,op.texts.image??''].filter(t=>t.trim()))].join('\n'):'',tasks:targets.map(target=>({target,text:op.texts[target]??'',annotation:target==='image'?null:(()=>{const a=d.annotations.find(a=>a.id===target);if(!a)return null;const {visible,instruction,...geometry}=a;return geometry;})()}))};
}
