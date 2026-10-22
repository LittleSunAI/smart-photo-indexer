import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useHelpPresentation} from './HelpPresentation';
import {Hint} from "../../shared/help/Hint";
import {Icon} from '../../shared/ui/Icon';
import {useId,useRef,useState} from 'react';
import {saveUiPreferences} from '../../shared/UiPreferences';
import {useUiPreferences} from '../../shared/useUiPreferences';
import './help-controls.css';

export function HelpButton({side='right'}:{side?:'left'|'right'}={}){
  useCurrentLocale();
 const presentation=useHelpPresentation();
 const preferences=useUiPreferences(),[error,setError]=useState<string|null>(null),[open,setOpen]=useState(false);
 const revealed=open||Boolean(presentation.view.helpDrawer);
 const trigger=useRef<HTMLButtonElement>(null),id=useId();
 const help=<Hint message={{id:'help.open'}}><button type="button" className="help-trigger" aria-label={localeText("main", "ui.HelpButton.d83f1e00ca07c808")} title={localeText("main", "ui.HelpButton.0e2c5b8b27e68d46")} onClick={()=>window.dispatchEvent(new Event('smart-photo-indexer:open-help'))}><Icon name="help"/></button></Hint>;
 const disclosure=<Hint message={{id:'help.hints-reveal'}} availableWhenDisabled><button ref={trigger} type="button" className="help-controls__disclosure" aria-label={localeText("main", "ui.HelpButton.d94dd98ca4d106ee")} aria-expanded={revealed} aria-controls={revealed?id:undefined} onClick={()=>setOpen(value=>!value)}><Icon name={(side==='right')!==revealed?'right':'left'}/></button></Hint>;
 const setting=revealed?<span id={id} className="help-controls__settings"><Hint message={{id:'help.hints-toggle'}} availableWhenDisabled><label className="help-controls__option" title={localeText("main", "ui.HelpButton.48dd87235ddd7450")}><input type="checkbox" aria-label={localeText("main", "ui.HelpButton.dc069db0494bb187")} data-help-id="hover-hints-toggle" checked={preferences.hoverHintsEnabled} onChange={event=>{try{saveUiPreferences({hoverHintsEnabled:event.target.checked});setError(null);}catch{setError(localeText("main", "ui.HelpButton.66f089689d2edd3c"));}}}/></label></Hint></span>:null;
 return <span className="help-controls" data-side={side} data-expanded={revealed} data-tour-id="main.help.overview main.help.hover editor.top.help viewer.top.help" data-help-id="help-hints-control" onKeyDown={event=>{if(event.key==='Escape'&&open){event.preventDefault();event.stopPropagation();setOpen(false);trigger.current?.focus();}}}>
  {side==='right'?help:null}
  <span className="help-controls__drawer">{side==='left'?disclosure:setting}{side==='left'?setting:disclosure}</span>
  {side==='left'?help:null}
  {error?<span className="help-controls__error" role="alert">{error}</span>:null}
 </span>;
}
