import{useEffect,useState}from'react';
import{useAppSelector}from'../../app/state/AppStateContext';
import{useHelpPresentation}from'../help/HelpPresentation';
import{SettingsPanel}from'./SettingsPanel';
/** Keep one settings session after first opening; secrets stay only in component memory. */
export function SettingsHost(){const normalOpen=useAppSelector(s=>s.settingsPanelOpen),help=useHelpPresentation();const open=help.open?Boolean(help.view.settingsOpen):normalOpen;const[visited,setVisited]=useState(false);useEffect(()=>{if(open)setVisited(true);},[open]);if(!open&&!visited)return null;return <div className="settings-overlay settings-overlay--tabs" role="presentation" hidden={!open}><div className="settings-overlay__panel"><SettingsPanel open={open}/></div></div>;}
