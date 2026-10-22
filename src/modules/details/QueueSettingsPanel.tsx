import{requestSettingsTab}from'../settings/SettingsNavigation';
import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useTourDisclosure} from '../help/HelpPresentation';
import {Hint} from "../../shared/help/Hint";
import {useEffect,useState} from 'react';
import {useAppActions} from '../../app/state/AppStateContext';
import {Icon} from '../../shared/ui/Icon';
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';
import {nativeQueueService} from '../taskQueue/NativeQueueService';
import type {TaskQueueState} from '../taskQueue/types';
import {formatCostUsd} from '../taskQueue/TaskQueueService';

const providers:Record<string,string>={get local() { return localeText("details", "ui.QueueSettingsPanel.a9ead2f59404a3b4"); },fal:'Fal AI',openrouter:'OpenRouter'};
export function QueueSettingsPanel({queue}:{queue:TaskQueueState}){
  useCurrentLocale();
  const actions=useAppActions(),disclosure=useTourDisclosure('queue-settings');
  const current=queue.config.requestedConcurrency??queue.config.concurrency??1;
  const [draft,setDraft]=useState(current),[busy,setBusy]=useState(false),[error,setError]=useState('');
  useEffect(()=>{setDraft(current);setError('');},[queue.nativeJobId,current]);
  const locked=!queue.nativeJobId||['completed','stopped'].includes(queue.nativeStatus??'');
  const apply=async()=>{if(!queue.nativeJobId)return;setBusy(true);setError('');try{await nativeQueueService.setConcurrency(queue.nativeJobId,draft);}catch(e){setError(String(e));}finally{setBusy(false);}};
  return <details {...disclosure} data-tour-id="main.queue.parameters" data-help-id="queue-settings" className="details-collapsible-card task-settings">
    <Hint message={{id:"queue.settings"}}><summary><HintTitle title={localeText("details", "ui.QueueSettingsPanel.7806ae44c03993ae")}  message={{id:"queue.settings"}}/><Icon name="down"/></summary></Hint>
    <div className="details-collapsible-card__body">
      <div className="queue-model-line" title={queue.config.queueMode==='direct'?localeText("details", "ui.QueueSettingsPanel.1576c7ee650de6ca"):undefined}><strong>{providers[queue.config.providerMode]??queue.config.providerMode}</strong><span title={queue.config.model}>{queue.config.model}</span></div>
      <dl className="queue-parameters">
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.ea5fb5a5dd5b34fe")}</dt><dd>{queue.config.outputMode==='tags'?localeText("details", "ui.QueueSettingsPanel.3c4f739b4ac7ddeb"):queue.config.outputMode==='description'?localeText("details", "ui.QueueSettingsPanel.b830b91a3dfa34a1"):localeText("details", "ui.QueueSettingsPanel.5cf43bf0d052bf58")}</dd></div>
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.b1840eaec40ab42c")}</dt><dd>{queue.nativeCounters?.total??queue.tasks.length}</dd></div>
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.4f1f4c361966a7e7")}</dt><dd>{queue.config.passes??1}</dd></div>
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.e2ddbcabf7f8d039")}</dt><dd>{(queue.config.queueContractVersion??0)>=2&&queue.config.maxEstimatedCostUsd===0?localeText("details", "ui.QueueSettingsPanel.616fc7da458d053d"):formatCostUsd(queue.config.maxEstimatedCostUsd)}</dd></div>
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.7243f2e3a7539100")}</dt><dd>{queue.config.requestDelayMs} {' '}{localeText("details", "ui.QueueSettingsPanel.1be883ab9ecb61ca")}</dd></div>
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.663f0ed11a2cf68b")}</dt><dd>{queue.config.maxRetriesPerTask}</dd></div>
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.a950bddcb796715b")}</dt><dd>{queue.config.effectiveConcurrency??1} {' '}{localeText("details", "ui.QueueSettingsPanel.319d98abab0ac404")}</dd></div>
        <div><dt>{localeText("details", "ui.QueueSettingsPanel.dbf0d50223b7768d")}</dt><dd title={nativeDisplayText(queue.config.concurrencyPolicy??'')}>{queue.config.occupiedRequests??0} {' '}{localeText("details", "ui.QueueSettingsPanel.bfff5fde6ac39537")}</dd></div>
      </dl>
      {queue.nativeJobId?<div className="queue-concurrency-control">
        <label htmlFor="queue-concurrency">{localeText("details", "ui.QueueSettingsPanel.e09853dbb411d6ad")}</label>
        <Hint message={{id:"queue.concurrency"}} supplement={locked||queue.config.providerMode==='local'?{id:"queue.concurrency-disabled"}:undefined}><input id="queue-concurrency" aria-label={localeText("details", "ui.QueueSettingsPanel.80b3349c85dccacb")} type="number" min={1} max={queue.config.providerMode==='local'?1:100} value={draft} disabled={busy||locked||queue.config.providerMode==='local'} onChange={event=>setDraft(Math.max(1,Math.min(100,Number(event.target.value)||1)))}/></Hint>
        <Hint message={{id:"queue.concurrency-apply"}}><button type="button" className="secondary-action" aria-label={localeText("details", "ui.QueueSettingsPanel.04b5c6042734b4f9")} title={localeText("details", "ui.QueueSettingsPanel.bef02a1b94225ec4")} disabled={busy||locked||draft===current} onClick={()=>void apply()}><Icon name="check"/>{localeText("details", "ui.QueueSettingsPanel.a43cc49b7bffca65")}</button></Hint>
        <small>{localeText("details", "ui.QueueSettingsPanel.989a4c5c616771d4")}</small>
      </div>:null}
      {error?<p role="alert" className="danger">{error}</p>:null}
      <div className="queue-options-summary"><span>{localeText("details", "ui.QueueSettingsPanel.4f23d8a53a3d8818")}{' '}{queue.config.stopOnFirstError?localeText("details", "ui.QueueSettingsPanel.b48197970f7ec239"):localeText("details", "ui.QueueSettingsPanel.d161135c6ea6d8af")}</span><Hint message={{id:"queue.new-settings"}}><button type="button" className="secondary-action queue-new-settings" onClick={()=>{requestSettingsTab('queue');actions.setSettingsPanelOpen(true);}}><Icon name="settings"/>{localeText("details", "ui.QueueSettingsPanel.22f429dd0fb9523f")}</button></Hint></div>
    </div>
  </details>;
}
