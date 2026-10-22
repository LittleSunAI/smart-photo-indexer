import{nativeDisplayText}from'../../shared/i18n/NativeMessages';
import{useState}from'react';
import{invoke}from'../../shared/tauri/invoke';
import{text,useCurrentLocale}from'../../shared/i18n/Runtime';
import{displayNumber,localeTag}from'../../shared/i18n/Formatting';
interface Preview{token:string;eligibleCount:number;estimatedBytes:number;deferred:boolean;cutoffUnix:number|null;zone:string|null;protected:{id:string;title:string;reason:string}[];projects:{id:string;title:string}[]}
interface Result{deletedProjects:number;reclaimedBytes:number;removedFiles:number;estimatedBytes:number;deferred:boolean;errors:{assetId:string;message:string}[]}
export const projectBytes=(bytes:number)=>`${displayNumber(bytes/1048576,{maximumFractionDigits:2})} ${text('editor','cleanup.mib')}`;
export function ProjectCleanup({selectedPaths=[],initialOpen=false,onChanged,onBusyChange}:{selectedPaths?:string[];initialOpen?:boolean;onChanged():void;onBusyChange?(busy:boolean):void}){
 useCurrentLocale();const[scope,setScope]=useState('all'),[date,setDate]=useState(''),[preview,setPreview]=useState<Preview|null>(null),[result,setResult]=useState<Result|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const pathsKey=JSON.stringify(selectedPaths);const[previewPaths,setPreviewPaths]=useState('');const zone=Intl.DateTimeFormat().resolvedOptions().timeZone;
 const run=async(work:()=>Promise<void>)=>{setBusy(true);onBusyChange?.(true);setError('');try{await work();}catch(e){setError(String(e));}finally{setBusy(false);onBusyChange?.(false);}};
 const change=()=>{setPreview(null);setResult(null);setError('');};
 return <details className="editor-project-cleanup" open={initialOpen||undefined} data-tour-id="main.library.projects.cleanup" data-testid="project-cleanup"><summary>{text('editor','cleanup.title')}</summary><div className="project-cleanup-controls">
 <label>{text('editor','cleanup.scope')}<select disabled={busy} value={scope} onChange={e=>{setScope(e.target.value);change();}}><option value="all">{text('editor','cleanup.all')}</option><option value="selected" disabled={!selectedPaths.length}>{text('editor','cleanup.selected')}</option><option value="before">{text('editor','cleanup.before')}</option></select></label>
 {scope==='before'?<label>{text('editor','cleanup.date')}<input type="date" value={date} disabled={busy} onChange={e=>{setDate(e.target.value);change();}}/><small>{zone}</small></label>:null}
 <button type="button" disabled={busy||scope==='before'&&!date||scope==='selected'&&!selectedPaths.length} onClick={()=>void run(async()=>{setPreview(null);setResult(null);setPreviewPaths(pathsKey);setPreview(await invoke<Preview>('preview_project_cleanup',{scope:{kind:scope,paths:scope==='selected'?selectedPaths:[],beforeDate:scope==='before'?date:null,zone:scope==='before'?zone:null}}));})}>{text('editor','cleanup.preview')}</button>
 {preview&&previewPaths===pathsKey?<div className="project-cleanup-preview" data-testid="project-cleanup-preview"><strong>{text('editor','cleanup.estimate',{count:displayNumber(preview.eligibleCount),bytes:projectBytes(preview.estimatedBytes)})}</strong>
 {preview.cutoffUnix!==null?<p>{text('editor','cleanup.boundary',{date:new Date(preview.cutoffUnix*1000).toLocaleString(localeTag(),{timeZone:preview.zone??zone}),zone:preview.zone??zone})}</p>:null}
 {preview.protected.length?<p>{text('editor','cleanup.protected',{count:displayNumber(preview.protected.length)})} {preview.protected.map(p=>p.title).join(', ')}</p>:null}
 {preview.eligibleCount?<><p className="project-cleanup-names">{preview.projects.filter(p=>!preview.protected.some(b=>b.id===p.id)).map(p=>p.title).join(', ')}</p><p>{text('editor','cleanup.warning')}</p><div className="project-cleanup-actions"><button type="button" disabled={busy} className="danger" onClick={()=>void run(async()=>{const token=preview.token;setPreview(null);setResult(await invoke<Result>('confirm_project_cleanup',{token}));onChanged();})}>{text('editor','cleanup.confirm')}</button><button type="button" disabled={busy} onClick={()=>setPreview(null)}>{text('editor','cleanup.cancel')}</button></div></>:<p>{text('editor','cleanup.empty')}</p>}</div>:null}
 {result?<div role="status" data-testid="project-cleanup-result"><p>{text('editor','cleanup.result',{count:displayNumber(result.deletedProjects),bytes:projectBytes(result.reclaimedBytes),files:displayNumber(result.removedFiles)})}</p>{result.deferred||result.errors.length?<p>{text('editor','cleanup.partial')}</p>:null}{result.errors.map(e=><p key={e.assetId} role="alert">{nativeDisplayText(e.message)}</p>)}</div>:null}
 {busy?<p role="status">{text('editor','cleanup.busy')}</p>:null}{error?<p role="alert">{nativeDisplayText(error)}</p>:null}
 </div></details>;
}
