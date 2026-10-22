import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {isQueryView} from '../../app/state/QuickSearch';
import {useState} from 'react';
import {useAppActions,useAppSelector} from '../../app/state/AppStateContext';
import {selectSelectedImageItems} from '../../app/state/appSelectors';
import {Icon} from '../../shared/ui/Icon';
import {EditorWorkbenchDialog} from './EditorWorkbenchDialog';
export function EditSelectedButton(){
  useCurrentLocale();
  const actions=useAppActions();
  const count=useAppSelector(s=>isQueryView(s)&&s.search.selectionSnapshot?s.search.selectionSnapshot.totalCount-s.search.selectionSnapshot.excludedPaths.length:selectSelectedImageItems(s).length);
  const [busy,setBusy]=useState(false),[projects,setProjects]=useState(false);
  return <><div className="editor-access"><Hint message={{id:"main.edit-photo"}} supplement={count!==1?{id:"main.edit-one-required"}:undefined}><button data-tour-id="main.commands.edit" type="button" className="toolbar-edit-photo" aria-label={localeText("editor", "ui.EditSelectedButton.f8938750ff7f0309")} title={count===1?localeText("editor", "ui.EditSelectedButton.8225f7eb6a5bf4ae"):localeText("editor", "ui.EditSelectedButton.dc6fa58eaebc7ca9")} disabled={count!==1||busy} aria-busy={busy} onClick={()=>{setBusy(true);void actions.openSelectedPhotoEditor().finally(()=>setBusy(false));}}><Icon name="edit"/><span>{localeText("editor", "ui.EditSelectedButton.35bbc10004329436")}</span></button></Hint><Hint message={{id:"main.editor-projects"}}><button type="button" aria-label={localeText("editor", "ui.EditSelectedButton.f84cc669a94ffc87")} title={localeText("editor", "ui.EditSelectedButton.585458da6f357506")} onClick={()=>setProjects(true)}><Icon name="down"/></button></Hint></div>{projects?<EditorWorkbenchDialog onClose={()=>setProjects(false)}/>:null}</>;
}
