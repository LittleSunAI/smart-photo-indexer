import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {outputHelp} from '../../shared/help/SelectionHelp';
import {Hint} from "../../shared/help/Hint";
import {useAppDispatch,useAppSelector} from '../../app/state/AppStateContext';
import {Icon} from '../../shared/ui/Icon';
import type {AnalysisOutputMode} from './Settings.types';
import {useFixedSelectWidth} from '../../shared/ui/useFixedSelectWidth';

export function AnalysisOutputSelect({compact=false}:{compact?:boolean}){
  useCurrentLocale();
  const mode=useAppSelector(state=>state.settings.processingLimits.outputMode),dispatch=useAppDispatch();
  const width=useFixedSelectWidth(localeText("settings", "ui.AnalysisOutputSelect.0298784aae5c7fe4"));
  const control=<Hint message={{id:"main.output"}} supplement={outputHelp(mode)}><select data-tour-id="main.recognition.output" ref={width.ref} title={mode==='tags'?localeText("settings", "ui.AnalysisOutputSelect.46957b9cc7628717"):mode==='description'?localeText("settings", "ui.AnalysisOutputSelect.f625b00c41397258"):localeText("settings", "ui.AnalysisOutputSelect.01b0bda060ea2d94")} aria-label={localeText("settings", "ui.AnalysisOutputSelect.6783bc6a9c8d596e")} value={mode} onChange={event=>dispatch({type:'settings/setAnalysisOutput',value:event.target.value as AnalysisOutputMode})}>
      <option value="both">{localeText("settings", "ui.AnalysisOutputSelect.690119be5779b631")}</option><option value="tags">{localeText("settings", "ui.AnalysisOutputSelect.274fe745a4400677")}</option><option value="description">{localeText("settings", "ui.AnalysisOutputSelect.ceea4da0743706f5")}</option>
    </select></Hint>;
  return compact?<div data-help-id="recognition-output" className="analysis-output-select"><span>{localeText("settings", "ui.AnalysisOutputSelect.35a0102dc21bd7b2")}</span><label className="toolbar-select-control toolbar-select-control--output" style={width.style}><Icon name="tag"/>{control}<span className="toolbar-select-chevron"><Icon name="down"/></span></label></div>:<label className="settings-field"><span>{localeText("settings", "ui.AnalysisOutputSelect.2ae2828ba051206f")}</span>{control}</label>;
}
