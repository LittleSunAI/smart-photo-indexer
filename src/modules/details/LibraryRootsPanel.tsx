import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect,useRef,useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {listen} from '@tauri-apps/api/event';
import { fileSystemService } from '../filesystem/FileSystemService';
import { useAppActions } from '../../app/state/AppStateContext';
import { isTauriRuntime } from '../../shared/tauri/runtime';
import {InfoHint} from '../../shared/ui/InfoHint';
import {Icon} from '../../shared/ui/Icon';
import {EditorErrorDialog} from '../photoEditor/EditorErrorDialog';

interface Root {id:string;path:string;excludedPaths:string[];autoUpdate:boolean;revision:number;status:{state:string;error?:string;checked:number;skipped:number;watching:boolean}}
const labels:Record<string,string>={get scanning() { return localeText("details", "ui.LibraryRootsPanel.0b3c49038e9340a5"); },get ready() { return localeText("details", "ui.LibraryRootsPanel.44b7ad3cdd31da8d"); },get partial() { return localeText("details", "ui.LibraryRootsPanel.687e0b745940ce4c"); },get unavailable() { return localeText("details", "ui.LibraryRootsPanel.2742f42155610743"); }};
export function LibraryRootsPanel(){
  useCurrentLocale();
  const [roots,setRoots]=useState<Root[]>([]),[error,setError]=useState(''),[reloadError,setReloadError]=useState(''),[busy,setBusy]=useState(false);
  const alive=useRef(false),generation=useRef(0),pending=useRef<Promise<void>|null>(null),reloadQueued=useRef(false);const actions=useAppActions();
  async function reload(force=false){
    if(pending.current){if(force)reloadQueued.current=true;await pending.current;return;}
    const id=++generation.current;
    const request=(async()=>{do{reloadQueued.current=false;const result=await invoke<{roots:Root[]}>('get_library_roots');if(alive.current&&generation.current===id)setRoots(previous=>JSON.stringify(previous)===JSON.stringify(result.roots)?previous:result.roots);}while(reloadQueued.current&&alive.current);})();
    pending.current=request;try{await request;}finally{if(pending.current===request)pending.current=null;}
  }
  useEffect(()=>{if(!isTauriRuntime())return;alive.current=true;let off:(()=>void)|undefined;
    const refresh=()=>{void reload(true).then(()=>{if(alive.current)setReloadError('');}).catch(e=>{if(alive.current)setReloadError(String(e));});};
    void listen('smart-photo-indexer:library-roots-changed',refresh).then(stop=>{if(!alive.current)stop();else{off=stop;refresh();}});
    window.addEventListener('focus',refresh);
    return()=>{alive.current=false;generation.current++;off?.();window.removeEventListener('focus',refresh);};
  },[]);
  async function run(work:()=>Promise<unknown>){setBusy(true);setError('');setReloadError('');try{await work();await reload(true);}catch(e){if(alive.current)setError(String(e));}finally{if(alive.current)setBusy(false);}}
  if(!isTauriRuntime())return null;
   return <section className="details-section" aria-label={localeText("details", "ui.LibraryRootsPanel.dd4bbeb4ad56d99d")}>
    <p className="metadata-edit-note">{localeText("details", "ui.LibraryRootsPanel.46a0da1f3c53d622")}</p>
    <div className="library-root-actions"><Hint message={{id:"library.connect"}}><button type="button" className="secondary-action" disabled={busy} onClick={()=>{void run(async()=>{const path=await fileSystemService.chooseFolder();if(path)await invoke('add_library_root',{path});});}}>{localeText("details", "ui.LibraryRootsPanel.23af897c3b488733")}</button></Hint><InfoHint label={localeText("details", "ui.LibraryRootsPanel.5aa49a53b8ae1945")}  message={{id:"library.connect"}}/></div>
    {roots.length===0?<p>{localeText("details", "ui.LibraryRootsPanel.4174af5fd92f9ea9")}</p>:null}
    {roots.map(root=><RootCard key={root.id} root={root} busy={busy} run={run} open={()=>{void actions.openPath(root.path);}}/>)}
    {error||reloadError?<EditorErrorDialog title={localeText("details", "ui.LibraryRootsPanel.e74cf4df06ca1f2b")} message={error||reloadError} onClose={()=>{setError('');setReloadError('');}}/>:null}
  </section>;
}
function RootCard({root,busy,run,open}:{root:Root;busy:boolean;run(work:()=>Promise<unknown>):Promise<void>;open():void}){
  useCurrentLocale();
  const [exclusions,setExclusions]=useState(root.excludedPaths.join('\n')),[automatic,setAutomatic]=useState(root.autoUpdate),[base,setBase]=useState(root.revision);
  const dirty=exclusions!==root.excludedPaths.join('\n') || automatic!==root.autoUpdate;
  useEffect(()=>{if(!dirty){setExclusions(root.excludedPaths.join('\n'));setAutomatic(root.autoUpdate);setBase(root.revision);}},[root.revision,dirty]);
  return <details className="details-collapsible-card library-root-card"><Hint message={{id:"library.root-open"}}><summary title={root.path}><span className="library-root-title">{root.path.split(/[\\/]/).filter(Boolean).pop()} · {labels[root.status.state]??localeText("details", "ui.LibraryRootsPanel.8bcc874240bed55c")}</span><Icon name="down"/><Hint message={{id:"library.disconnect"}}><button type="button" className="library-root-disconnect" aria-label={localeText("details", "ui.LibraryRootsPanel.87b625c0dcbe7e3d", {"value1": (root.path)})} title={localeText("details", "ui.LibraryRootsPanel.eeb30616930f2228")} disabled={busy} onClick={event=>{event.preventDefault();event.stopPropagation();void run(()=>invoke('remove_library_root',{rootId:root.id}));}} onKeyDown={event=>{if(event.key==='Enter'||event.key===' ')event.stopPropagation();}}><Icon name="close"/></button></Hint></summary></Hint>
    <div className="details-collapsible-card__body"><span style={{overflowWrap:'anywhere'}}>{root.path}</span>
      {root.status.state==='ready' || root.status.state==='partial'?<span>{localeText("details", "ui.LibraryRootsPanel.2e47c6fc417aa295")}{' '}{root.status.checked} {' '}{localeText("details", "ui.LibraryRootsPanel.9cc29d4d12376e47")}{' '}{root.status.skipped}</span>:null}
      {root.status.error?<span role="status">{nativeDisplayText(root.status.error??'')}</span>:null}
      <Hint message={{id:"library.root-auto"}}><label><input type="checkbox" aria-label={localeText("details", "ui.LibraryRootsPanel.64294c3b9d1fd097", {"value1": (root.path)})} checked={automatic} disabled={busy} onChange={event=>setAutomatic(event.target.checked)}/> {' '}{localeText("details", "ui.LibraryRootsPanel.a415d3108ddcd7b6")}</label></Hint>
      <Hint message={{id:"library.exclusions"}}><label className="settings-field"><span>{localeText("details", "ui.LibraryRootsPanel.6b3597245d10b7fb")}</span><textarea aria-label={localeText("details", "ui.LibraryRootsPanel.c098facbf8492eef", {"value1": (root.path)})} value={exclusions} disabled={busy} onChange={event=>setExclusions(event.target.value)}/></label></Hint>
      <div className="library-root-save-actions"><Hint message={{id:"library.root-save"}}><button type="button" className="secondary-action" disabled={busy || !dirty} onClick={()=>{void run(async()=>{await invoke('update_library_root',{rootId:root.id,expectedRevision:base,excludedPaths:exclusions.split('\n').map(path=>path.trim()).filter(Boolean),autoUpdate:automatic});setBase(root.revision+1);});}}>{localeText("details", "ui.LibraryRootsPanel.d189d9d25a3d94d1")}</button></Hint><Hint message={{id:"common.reset-draft"}}><button type="button" className="secondary-action" aria-label={localeText("details", "ui.LibraryRootsPanel.a2b89ab7ddd6c46e")} title={localeText("details", "ui.LibraryRootsPanel.47b86c5766736880")} disabled={busy||!dirty} onClick={()=>{setExclusions(root.excludedPaths.join('\n'));setAutomatic(root.autoUpdate);setBase(root.revision);}}><Icon name="undo"/></button></Hint></div>
      <div className="queue-control-row"><Hint message={{id:"common.reveal-file"}}><button type="button" className="secondary-action" onClick={open}>{localeText("details", "ui.LibraryRootsPanel.b86dcd7e4ed156b2")}</button></Hint>
        <Hint message={{id:"library.root-refresh"}}><button type="button" className="secondary-action" disabled={busy || root.status.state==='scanning'} onClick={()=>{void run(()=>invoke('refresh_library_root',{rootId:root.id}));}}>{localeText("details", "ui.LibraryRootsPanel.0d10e743003314e3")}</button></Hint>
        <Hint message={{id:"library.disconnect"}}><button type="button" className="secondary-action" disabled={busy} onClick={()=>{void run(()=>invoke('remove_library_root',{rootId:root.id}));}}>{localeText("details", "ui.LibraryRootsPanel.120cb9eaee400a44")}</button></Hint></div>
      <small>{localeText("details", "ui.LibraryRootsPanel.6cf6b9154c1eb5cd")}</small>
    </div></details>;
}
