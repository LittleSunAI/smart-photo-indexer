import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useState} from 'react';
import {Hint} from '../../shared/help/Hint';
import {InfoHint} from '../../shared/ui/InfoHint';
import {saveUiPreferences} from '../../shared/UiPreferences';
import {useUiPreferences} from '../../shared/useUiPreferences';
import './hover-hints-setting.css';

export function HoverHintsSetting(){
  useCurrentLocale();
 const preferences=useUiPreferences(),[error,setError]=useState<string|null>(null);
 return <div className="hover-hints-setting" data-help-id="settings-hover-hints">
  <div className="hover-hints-setting__capsule">
   <Hint message={{id:'settings.hover-hints'}} availableWhenDisabled><label className="hover-hints-setting__label">
    <input type="checkbox" checked={preferences.hoverHintsEnabled} onChange={event=>{try{saveUiPreferences({hoverHintsEnabled:event.target.checked});setError(null);}catch{setError(localeText("settings", "ui.HoverHintsSetting.0d7c7fe42d719a3e"));}}}/>
    <span>{localeText("settings", "ui.HoverHintsSetting.8872bf27638e9186")}</span>
   </label></Hint>
   <InfoHint label={localeText("settings", "ui.HoverHintsSetting.710f6b1df68105e5")} message={{id:'settings.hover-hints'}}/>
  </div>
  {error?<p className="hover-hints-setting__error" role="alert">{error}</p>:null}
 </div>;
}
