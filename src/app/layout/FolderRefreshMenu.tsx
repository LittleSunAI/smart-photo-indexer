import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {useAppActions,useAppSelector} from '../state/AppStateContext';
import {Icon} from '../../shared/ui/Icon';
import {loadWorkspacePreferences,saveRefreshMode} from '../../modules/settings/WorkspacePreferences';
export function FolderRefreshMenu(){
  useCurrentLocale();
 const actions=useAppActions(),folderBusy=useAppSelector(s=>s.isScanningFolder),path=useAppSelector(s=>s.currentPath),indexBusy=useAppSelector(s=>s.indexDatabase.isInitializing);
 const [mode,setMode]=useState(()=>loadWorkspacePreferences().refreshMode),[busy,setBusy]=useState(false),[error,setError]=useState('');
 const caption=mode==='index'?localeText("main", "ui.FolderRefreshMenu.c0a2800191614085"):localeText("main", "ui.FolderRefreshMenu.40f6ffa05945ce72");
 const disabled=!path||busy||(mode==='index'?indexBusy:folderBusy);
 const anchor=useRef<HTMLDivElement>(null),menu=useRef<HTMLDivElement>(null);const [open,setOpen]=useState(false),[position,setPosition]=useState({top:0,left:0});
 const choose=(next:'folder'|'index')=>{try{saveRefreshMode(next);setMode(next);setError('');}catch{setError(localeText("main", "ui.FolderRefreshMenu.2cf35755d51ff9ae"));}setOpen(false);anchor.current?.querySelector<HTMLButtonElement>('.toolbar-refresh__menu-trigger')?.focus();};
 const execute=async()=>{setBusy(true);setError('');try{if(mode==='index')await actions.rescanIndex();else await actions.reloadFolder();}catch(e){setError(localeText("main", "ui.FolderRefreshMenu.905dbd67d980e0e6", {"value1": (String(e))}));}finally{setBusy(false);}};
 useEffect(()=>{if(!open)return;const outside=(e:PointerEvent)=>{if(!anchor.current?.contains(e.target as Node)&&!menu.current?.contains(e.target as Node))setOpen(false);};const close=()=>setOpen(false);document.addEventListener('pointerdown',outside);window.addEventListener('resize',close);return()=>{document.removeEventListener('pointerdown',outside);window.removeEventListener('resize',close);};},[open]);
 return <div ref={anchor} data-help-id="refresh" className="toolbar-refresh">
  <Hint message={{id:mode==='index'?'main.refresh-index':'main.refresh-folder'}} supplement={!path?{id:'main.no-folder-refresh'}:undefined}><button data-tour-id="main.folder.refresh" type="button" aria-label={caption} title={caption} disabled={disabled} aria-busy={busy} onClick={()=>void execute()}><Icon name={mode==='index'?'image':'refresh'}/><span className="toolbar-refresh__text">{mode==='index'?localeText("main", "ui.FolderRefreshMenu.e8737c5031f37c7a"):localeText("main", "ui.FolderRefreshMenu.509ad3af6213d1f5")}</span></button></Hint>
  <Hint message={{id:"main.refresh-mode"}}><button data-tour-id="main.folder.refresh-options" type="button" className="toolbar-refresh__menu-trigger" aria-label={localeText("main", "ui.FolderRefreshMenu.80ba2e84479abe51")} aria-haspopup="menu" aria-expanded={open} title={localeText("main", "ui.FolderRefreshMenu.57ed41f5868bd7d3")} onClick={()=>{const r=anchor.current?.getBoundingClientRect();if(r)setPosition({top:r.bottom+4,left:Math.max(8,Math.min(r.left,innerWidth-258))});setOpen(v=>!v);}}><Icon name="down"/></button></Hint>
  {error?<span className="toolbar-refresh__error" role="alert" title={error}>{error}</span>:null}
  {open?createPortal(<div ref={menu} role="menu" className="toolbar-refresh__menu" style={position} onKeyDown={e=>{if(e.key==='Escape'){setOpen(false);anchor.current?.querySelector<HTMLButtonElement>('.toolbar-refresh__menu-trigger')?.focus();}if(['ArrowDown','ArrowUp','Home','End'].includes(e.key)){e.preventDefault();const buttons=Array.from(menu.current!.querySelectorAll<HTMLButtonElement>('button'));const i=buttons.indexOf(document.activeElement as HTMLButtonElement);buttons[e.key==='Home'?0:e.key==='End'?buttons.length-1:(i+(e.key==='ArrowDown'?1:-1)+buttons.length)%buttons.length]?.focus();}}}>
   <Hint message={{id:"main.refresh-folder"}}><button data-tour-id="main.folder.refresh" type="button" role="menuitemradio" aria-checked={mode==='folder'} autoFocus onClick={()=>choose('folder')}><Icon name="refresh"/>{localeText("main", "ui.FolderRefreshMenu.4ebb13ebc49311d1")}{mode==='folder'?<Icon name="check"/>:null}</button></Hint>
   <Hint message={{id:"main.refresh-index"}}><button data-tour-id="main.folder.refresh" type="button" role="menuitemradio" aria-checked={mode==='index'} onClick={()=>choose('index')}><Icon name="image"/>{localeText("main", "ui.FolderRefreshMenu.dca9b606a536039f")}{mode==='index'?<Icon name="check"/>:null}</button></Hint>
  </div>,document.body):null}
 </div>;
}
