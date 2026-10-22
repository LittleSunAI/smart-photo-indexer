import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {listen} from '@tauri-apps/api/event';
import {useAppSelector} from '../../app/state/AppStateContext';
import {selectSelectionCost,selectionCostEqual} from '../../app/state/appSelectors';
import {formatCostUsd} from '../taskQueue/TaskQueueService';
import type {TaskQueueState} from '../taskQueue/types';
import {isTauriRuntime} from '../../shared/tauri/runtime';
import {Icon} from '../../shared/ui/Icon';
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';

interface ProviderCost {provider:string;attempts:number;spentUsd:number;reservedUsd:number;unknownCosts:number}
const labels:Record<string,string>={fal:'Fal AI',openrouter:'OpenRouter',get local() { return localeText("details", "ui.QueueCostSummary.a698771c8c4b6729"); }};

export function QueueSelectionCost(){
  useCurrentLocale();
  const cost=useAppSelector(selectSelectionCost,selectionCostEqual);
  if(!cost.selected)return null;
  return <div data-tour-id="main.queue.cost" className="queue-selection-cost" aria-label={localeText("details", "ui.QueueCostSummary.7570028f17e2dabf")}>
    <span>{cost.provider==='local'?localeText("details", "ui.QueueCostSummary.584fbaafcd9e381b"):<>{localeText("details", "ui.QueueCostSummary.566dcd76b0a3721a")}{' '}<strong>≈{formatCostUsd(cost.totalUsd)}</strong></>}</span>
    <span>{cost.count} {' '}{localeText("details", "ui.QueueCostSummary.cce16b2bbe3e18ee")}{' '}{cost.passes} {' '}{localeText("details", "ui.QueueCostSummary.1629def4e7fccc8b")}</span>
    {cost.selected>cost.count?<small>{localeText("details", "ui.QueueCostSummary.8286be5c930c37d6")}{' '}{cost.selected} {' '}{localeText("details", "ui.QueueCostSummary.a735e228ddb8a880")}</small>:null}
    {cost.provider!=='local'?<span className="queue-estimate-note" tabIndex={0} aria-label={localeText("details", "ui.QueueCostSummary.2794c11606dc1206")} title={localeText("details", "ui.QueueCostSummary.7b53e874a1bb1357")}><Icon name="info"/></span>:null}
    {cost.provider!=='local'&&cost.budgetUsd>0&&cost.totalUsd>cost.budgetUsd?<small className="queue-budget-warning">{localeText("details", "ui.QueueCostSummary.ff2c290aed50c246")}</small>:null}
  </div>;
}

export function QueueCostStatistics({queue}:{queue:TaskQueueState}){
  useCurrentLocale();
  const [statistics,setStatistics]=useState<ProviderCost[]|null>(null),[error,setError]=useState(false);
  useEffect(()=>{
    if(!isTauriRuntime())return;
    let cancelled=false,pending=false,dirty=false,timer:number|undefined,unlisten:(()=>void)|undefined,editorUnlisten:(()=>void)|undefined;
    const refresh=async()=>{
      if(cancelled)return;if(pending){dirty=true;return;}pending=true;
      try{const next=await invoke<ProviderCost[]>('get_processing_cost_statistics');if(!cancelled){setStatistics(previous=>JSON.stringify(previous)===JSON.stringify(next)?previous:next);setError(false);}}
      catch{if(!cancelled)setError(true);}
      finally{pending=false;if(dirty&&!cancelled){dirty=false;wake();}}
    };
    const wake=()=>{if(cancelled||timer!==undefined)return;timer=window.setTimeout(()=>{timer=undefined;void refresh();},1000);};
    void Promise.all([
      listen('processing-changed',wake).then(cleanup=>{if(cancelled)cleanup();else unlisten=cleanup;}),
      listen('smart-photo-indexer:editor-jobs-changed',wake).then(f=>cancelled?f():editorUnlisten=f),
    ]).then(()=>void refresh());
    window.addEventListener('smart-photo-indexer:processing-wake',wake);
    window.addEventListener('focus',wake);
    return()=>{cancelled=true;if(timer!==undefined)window.clearTimeout(timer);unlisten?.();editorUnlisten?.();window.removeEventListener('smart-photo-indexer:processing-wake',wake);window.removeEventListener('focus',wake);};
  },[]);
  const spent=statistics?.reduce((sum,row)=>sum+row.spentUsd,0)??0;
  const unknown=statistics?.reduce((sum,row)=>sum+row.unknownCosts,0)??0;
  return <details data-tour-id="main.queue.cost" className="queue-cost-statistics details-disclosure">
    <Hint message={{id:"queue.costs"}}><summary><HintTitle title={localeText("details", "ui.QueueCostSummary.c9b64d156c3d2d09")}  message={{id:"queue.costs"}}/><strong>{statistics===null?'…':formatCostUsd(spent)}{unknown?localeText("details", "ui.QueueCostSummary.6d13e381a96a3645"):''}</strong><Icon name="down"/></summary></Hint>
    <div className="queue-cost-statistics__body">
      <div className="queue-cost-total"><span>{localeText("details", "ui.QueueCostSummary.ee9a286fd0ded679")}</span><strong>{statistics===null?'…':formatCostUsd(spent)}</strong></div>
      {queue.nativeJobId?<div title={localeText("details", "ui.QueueCostSummary.b2be48b6aa7f218a", {"value1": (formatCostUsd(queue.estimatedPendingUsd??0))})}><span>{localeText("details", "ui.QueueCostSummary.3c6dcac0a0fe5237")}</span><strong>{formatCostUsd(queue.estimatedRunCostUsd)}</strong></div>:null}
      {queue.nativeJobId&&queue.reservedCostUsd?<p>{localeText("details", "ui.QueueCostSummary.5f3c77495808b468")}{' '}{formatCostUsd(queue.reservedCostUsd)}; {queue.unknownCosts??0} {' '}{localeText("details", "ui.QueueCostSummary.cf74e4fb977f79a9")}</p>:null}
      {statistics===null?<p>{error?localeText("details", "ui.QueueCostSummary.5031000f99fd6e9f"):localeText("details", "ui.QueueCostSummary.7fcae420661300f7")}</p>:statistics.length?statistics.map(row=><div className="provider-cost-row" key={row.provider}>
        <span>{labels[row.provider]??row.provider} · {row.attempts} {' '}{localeText("details", "ui.QueueCostSummary.bf95452c47440091")}</span><strong>{formatCostUsd(row.spentUsd)}</strong>
        {row.unknownCosts?<small>{localeText("details", "ui.QueueCostSummary.1a48ab0c116b3478")}{' '}{row.unknownCosts}{localeText("details", "ui.QueueCostSummary.3605f777d9db769e")}{' '}{formatCostUsd(row.reservedUsd)}.</small>:null}
      </div>):<p>{localeText("details", "ui.QueueCostSummary.6ddb5c56a84a0f0e")}</p>}
      {error&&statistics!==null?<p role="status">{localeText("details", "ui.QueueCostSummary.3cd0cbf26553b702")}</p>:null}
      <small>{localeText("details", "ui.QueueCostSummary.42d51acfb99c79d0")}</small>
    </div>
  </details>;
}
