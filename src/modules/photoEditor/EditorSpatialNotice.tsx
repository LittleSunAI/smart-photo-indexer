import{useState}from'react';
import{useCurrentLocale,text}from'../../shared/i18n/Runtime';
import{instructionRequest,localInstructions}from'./EditorInstructionState';
import type{EditorDraft}from'./EditorContracts';
export function EditorSpatialNotice({draft}:{draft:EditorDraft}){
 useCurrentLocale();
 const[dismissed,setDismissed]=useState<Set<string>>(()=>new Set());
 const request=instructionRequest(draft) as {tasks?:Array<{target:string;text:string;annotation?:{kind?:string}}> }|null;
 const local=request?.tasks?.filter(t=>t.annotation&&['point','rect'].includes(t.annotation.kind??''))??[];
 const key=draft.modelId??'';
 if(!localInstructions(draft.operation)||!key||!local.length||dismissed.has(key))return null;
 const gpt=['openai/gpt-image-2.5/flare/edit','openai/gpt-image-2.5/sunburst/edit'].includes(key);
 return <div className="editor-spatial-notice" data-testid="editor-spatial-notice" data-tour-id="editor.canvas.spatial-notice" role="status"><span>{text('editor',gpt?'spatial.noticeGpt':'spatial.noticeText')}</span><button type="button" aria-label={text('editor','spatial.dismiss')} onPointerDown={e=>e.stopPropagation()} onClick={()=>setDismissed(old=>new Set([...old,key]))}>×</button></div>;
}
