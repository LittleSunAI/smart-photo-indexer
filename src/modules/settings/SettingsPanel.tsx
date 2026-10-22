import{useEffect,useLayoutEffect,useRef,useState}from'react';
import{settingsTabs,useSettingsEntry,type SettingsTab}from'./SettingsNavigation';
import{loadUiPreferences,saveUiPreferences}from'../../shared/UiPreferences';
import{useHelpPresentation}from'../help/HelpPresentation';
import{AppearanceSettings}from'./AppearanceSettings';
import{ProcessingOptions}from'./ProcessingOptions';
import{EditorWorkbenchDialog}from'../photoEditor/EditorWorkbenchDialog';
import'./settings-tabs.css';
import {ProjectPreferencesFields} from '../photoEditor/ProjectPreferencesFields';
import {LanguageSetting} from './LanguageSetting';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {cloudPhotoConsentLabel} from './CloudPhotoConsentText';
import {AnalysisCopySettings} from './AnalysisCopySettings';
import { useAppActions, useAppDispatch, useAppState } from '../../app/state/AppStateContext';
import { NativeProviderSettings } from './NativeProviderSettings';
import { Icon } from '../../shared/ui/Icon';
import {QueueRecoverySettings} from './QueueRecoverySettings';
import {CloudConcurrencyField} from './CloudConcurrencyField';
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';

import {PhotoStorageSettings} from './PhotoStorageSettings';
import {HoverHintsSetting} from './HoverHintsSetting';

export function SettingsPanel({open=true}:{open?:boolean}) {
  useCurrentLocale();
  const state=useAppState(),actions=useAppActions(),dispatch=useAppDispatch(),help=useHelpPresentation(),entry=useSettingsEntry();
  const[tab,setTab]=useState<SettingsTab>(()=>entry.tab??loadUiPreferences().lastSettingsTab??'general'),[error,setError]=useState(''),[projects,setProjects]=useState(false);
  const currentEntry=useRef(0),root=useRef<HTMLElement>(null),body=useRef<HTMLDivElement>(null),priorTab=useRef<SettingsTab>(tab),offsets=useRef<Partial<Record<SettingsTab,number>>>({});
  const effectiveTab=help.view.settingsTab??(open&&entry.serial!==currentEntry.current&&entry.tab?entry.tab:tab),limits=state.settings.processingLimits;
  const selectTab=(next:SettingsTab)=>{if(help.open)return;setError('');setTab(next);try{saveUiPreferences({lastSettingsTab:next});}catch(e){setError(String(e));}};
  useEffect(()=>{if(open&&!help.open&&entry.serial!==currentEntry.current&&entry.tab){currentEntry.current=entry.serial;selectTab(entry.tab);}},[open,entry.serial,help.open]);
  useLayoutEffect(()=>{const el=body.current;if(!el)return;offsets.current[priorTab.current]=el.scrollTop;el.scrollTop=offsets.current[effectiveTab]??0;priorTab.current=effectiveTab;root.current?.querySelector<HTMLButtonElement>('#settings-tab-'+effectiveTab)?.scrollIntoView({block:'nearest',inline:'nearest'});},[effectiveTab,open]);
  const helpScroll=useRef<number|null>(null);
  useLayoutEffect(()=>{if(help.open)helpScroll.current=body.current?.scrollTop??0;else if(helpScroll.current!==null){if(body.current)body.current.scrollTop=helpScroll.current;helpScroll.current=null;}},[help.open]);
  useEffect(()=>{if(!open||help.open)return;const previous=document.activeElement as HTMLElement|null;root.current?.querySelector<HTMLButtonElement>('#settings-tab-'+effectiveTab)?.focus();return()=>{if(previous?.isConnected&&document.documentElement.dataset.helpActive!=='true')previous.focus();};},[open,help.open]);
  const pane=(name:SettingsTab)=>({id:'settings-pane-'+name,role:'tabpanel',hidden:effectiveTab!==name,inert:effectiveTab!==name||undefined,'aria-labelledby':'settings-tab-'+name,'data-settings-pane':name});
  return <section ref={root} className="settings-panel settings-panel--tabs" role="dialog" aria-modal={!help.open} data-help-overlay="settings" data-tour-id="settings.overview" aria-label={localeText('settings','ui.SettingsPanel.67ca623dd7565efa')} onKeyDown={event=>{if(help.open||!root.current?.contains(event.target as Node))return;if(event.key==='Escape'){event.preventDefault();event.stopPropagation();actions.setSettingsPanelOpen(false);}if(event.key==='Tab'){const fields=[...root.current.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]')].filter(e=>e.getClientRects().length&&!e.closest('[hidden],[inert]'));if(!fields.length)return;const first=fields[0],last=fields.at(-1);if(event.shiftKey&&document.activeElement===first){event.preventDefault();last?.focus();}else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}}}}>
    <div className="settings-panel__header"><strong>{localeText('settings','ui.SettingsPanel.f99061b50148f072')}</strong><nav className="settings-tab-track" role="tablist" aria-label={localeText('settings','ui.SettingsPanel.f99061b50148f072')}>{settingsTabs.map(value=><button key={value} type="button" id={'settings-tab-'+value} role="tab" aria-selected={effectiveTab===value} aria-controls={'settings-pane-'+value} tabIndex={effectiveTab===value?0:-1} onClick={()=>selectTab(value)} onKeyDown={event=>{if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;event.preventDefault();const index=settingsTabs.indexOf(value),next=event.key==='Home'?'general':event.key==='End'?'projects':settingsTabs[(index+(event.key==='ArrowRight'?1:4))%5];selectTab(next);root.current?.querySelector<HTMLButtonElement>('#settings-tab-'+next)?.focus();}}>{localeText('settings',('tabs.'+value) as 'tabs.general')}</button>)}</nav><Hint message={{id:'common.close-dialog'}}><button type="button" className="settings-close" aria-label={localeText('settings','ui.SettingsPanel.45bcd821ae3284e5')} onClick={()=>actions.setSettingsPanelOpen(false)}><Icon name="close"/></button></Hint></div>
    {error?<p role="alert" className="settings-session-error">{error}</p>:null}
    <div ref={body} className="settings-panel__body">
      <div className="settings-pane" {...pane('general')}><section className="settings-card" data-tour-id="settings.general"><h3>{localeText('settings','tabs.general')}</h3><LanguageSetting/><AppearanceSettings/><HoverHintsSetting/>
        <Hint message={{id:'settings.recursive-view'}}><label className="settings-toggle" data-tour-id="settings.general.subfolders"><input type="checkbox" checked={state.includeSubfolders} onChange={e=>actions.setIncludeSubfolders(e.target.checked)}/>{localeText('settings','ui.SettingsPanel.e34b7449c9375832')}</label></Hint>
        <Hint message={{id:'settings.images-only'}}><label className="settings-toggle" data-tour-id="settings.general.files"><input type="checkbox" checked={state.showImagesOnly} onChange={e=>actions.setShowImagesOnly(e.target.checked)}/>{localeText('settings','ui.SettingsPanel.48a6cbecfd1fad90')}</label></Hint>
      </section></div>
      <div className="settings-pane" {...pane('recognition')} data-tour-id="settings.recognition"><NativeProviderSettings active={open}/><section className="settings-card"><ProcessingOptions surface="settings"/><Hint message={{id:'settings.cloud-consent'}}><label className="settings-toggle"><input type="checkbox" checked={state.aiConsentAccepted} onChange={e=>actions.setAiConsent(e.target.checked)}/>{cloudPhotoConsentLabel()}</label></Hint></section></div>
      <div className="settings-pane" {...pane('queue')} data-tour-id="settings.queue">
      <section className="settings-card">
        <h3><HintTitle title={localeText("settings", "ui.SettingsPanel.28f2d3fa3d2a8b08")}  message={{id:"queue.section"}}/></h3>
        <Hint message={{id:"main.mode-manual"}}><label className="settings-toggle"><input type="checkbox" checked={limits.manualQueue} onChange={event=>dispatch({type:'settings/setManualQueue',value:event.target.checked})}/>{localeText("settings", "ui.SettingsPanel.d4bbd4c78ff3e5ad")}</label></Hint>
        <div className="settings-number-grid">

          <Hint message={{id:"settings.request-delay"}}><label className="settings-field"><span>{localeText("settings", "ui.SettingsPanel.f702b7f3b76c62e7")}</span><input type="number" min={0} max={60000} value={limits.requestDelayMs} onChange={event => actions.setRequestDelayMs(Number(event.target.value))} /></label></Hint>
          <Hint message={{id:"settings.job-budget"}}><label className="settings-field" title={localeText("settings", "ui.SettingsPanel.f1b660cf8706c852")}><span>{localeText("settings", "ui.SettingsPanel.56a525308d9817c1")}</span><input type="number" min={0} max={1000} step={0.1} value={limits.maxEstimatedCostPerRunUsd} onChange={event => actions.setMaxCostUsd(Number(event.target.value))} /></label></Hint>
          <Hint message={{id:"settings.retries"}}><label className="settings-field"><span>{localeText("settings", "ui.SettingsPanel.c0d22541902ea3cf")}</span><input type="number" min={0} max={5} value={limits.maxRetriesPerTask} onChange={event => actions.setMaxRetriesPerTask(Number(event.target.value))} /></label></Hint>
          <Hint message={{id:"settings.passes"}}><label className="settings-field"><span>{localeText("settings", "ui.SettingsPanel.5548283d3dbfa260")}</span><input type="number" min={1} max={5} value={limits.passes} onChange={event => dispatch({ type: 'settings/setAnalysisLimit', name: 'passes', value: Number(event.target.value) })} /></label></Hint>
          <CloudConcurrencyField/>
        </div>
        <p className="settings-muted">{localeText("settings", "ui.SettingsPanel.35fbb4b0033b4e88")}</p>
        <Hint message={{id:"settings.pause-ai-error"}}><label className="settings-toggle"><input type="checkbox" checked={limits.stopOnFirstAiError} onChange={event => actions.setStopOnFirstAiError(event.target.checked)} />{localeText("settings", "ui.SettingsPanel.7b15fcacad74001c")}</label></Hint>
        <Hint message={{id:"settings.pause-rename-error"}}><label className="settings-toggle"><input type="checkbox" checked={state.settings.rename.stopOnFirstError} onChange={event => actions.setRenameStopOnError(event.target.checked)} />{localeText("settings", "ui.SettingsPanel.8c436b7ffe478d43")}</label></Hint>
        <details><Hint message={{id:"common.expand",args:{раздел:localeText("settings", "ui.SettingsPanel.899f88a05da6a5ae")}}}><summary>{localeText("settings", "ui.SettingsPanel.03f42881e8d3b070")}</summary></Hint>
          <Hint message={{id:"settings.tokens"}}><label className="settings-field"><span>{localeText("settings", "ui.SettingsPanel.50d756c7c4eea47c")}</span><input type="number" min={1600} max={4096} value={limits.maxTokens} onChange={event => dispatch({ type: 'settings/setAnalysisLimit', name: 'maxTokens', value: Number(event.target.value) })} /></label></Hint>
          <Hint message={{id:"settings.image-side"}}><label className="settings-field"><span>{localeText("settings", "ui.SettingsPanel.27719084bb57f0dd")}</span><input type="number" min={1024} max={4096} value={limits.imageMaxSide} onChange={event => dispatch({ type: 'settings/setAnalysisLimit', name: 'imageMaxSide', value: Number(event.target.value) })} /></label></Hint>
          <p className="settings-muted">{localeText("settings", "ui.SettingsPanel.19d36677a4d2de99")}</p>
        </details>
      </section>
        <QueueRecoverySettings/>
      </div>
      <div className="settings-pane" {...pane('storage')} data-tour-id="settings.storage"><section className="settings-card"><h3>{localeText('settings','tabs.analysisCopies')}</h3><AnalysisCopySettings/></section><PhotoStorageSettings/></div>
      <div className="settings-pane" {...pane('projects')} data-tour-id="settings.projects"><section className="settings-card"><ProjectPreferencesFields/></section><section className="settings-card"><p className="settings-muted">{localeText('settings','tabs.projectsNote')}</p><button type="button" onClick={()=>setProjects(true)}>{localeText('editor','cleanup.manage')}</button></section></div>
    </div>
    {projects?<EditorWorkbenchDialog onClose={()=>setProjects(false)}/>:null}
  </section>;
}
