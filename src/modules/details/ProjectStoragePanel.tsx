import{displayNumber}from'../../shared/i18n/Formatting';
import{nativeDisplayText}from'../../shared/i18n/NativeMessages';
import{useEffect,useState}from'react';
import{useAppSelector}from'../../app/state/AppStateContext';
import{selectSelectedImageItems}from'../../app/state/appSelectors';
import{isQueryView}from'../../app/state/QuickSearch';
import{invoke}from'../../shared/tauri/invoke';
import{text,useCurrentLocale}from'../../shared/i18n/Runtime';
import{EditorWorkbenchDialog}from'../photoEditor/EditorWorkbenchDialog';
import{projectBytes}from'../photoEditor/ProjectCleanup';
export function ProjectStoragePanel(){useCurrentLocale();const[storage,setStorage]=useState<{count:number;ownedBytes:number}|null>(null),[error,setError]=useState(''),[open,setOpen]=useState(false),[refresh,setRefresh]=useState(0);const paths=useAppSelector(s=>JSON.stringify(isQueryView(s)&&s.search.selectionSnapshot?[]:selectSelectedImageItems(s).map(p=>p.path)));
 useEffect(()=>{let alive=true;void invoke<{storage:{count:number;ownedBytes:number}}>('get_editor_workbench').then(v=>{if(alive){setStorage(v.storage);setError('');}}).catch(e=>{if(alive)setError(String(e));});return()=>{alive=false;};},[refresh]);
 return <div data-testid="project-storage"><p>{storage?text('editor','cleanup.summary',{count:displayNumber(storage.count),bytes:projectBytes(storage.ownedBytes)}):text('editor','cleanup.loading')}</p><button type="button" className="secondary-action" onClick={()=>setOpen(true)}>{text('editor','cleanup.manage')}</button>{error?<p role="alert">{nativeDisplayText(error)}</p>:null}{open?<EditorWorkbenchDialog selectedPaths={JSON.parse(paths)} cleanupInitiallyOpen onClose={()=>{setOpen(false);setRefresh(v=>v+1);}}/>:null}</div>;
}
