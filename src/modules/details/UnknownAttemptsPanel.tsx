import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';
import { useEffect, useRef, useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
interface Attempt { id: string; path: string; passNumber: number; status: string; remoteId: string | null; reservedUsd: number; costUsd: number | null }
interface Page { items: Attempt[]; totalCount: number; nextOffset: number | null }
export function UnknownAttemptsPanel({ jobId, status, count }: { jobId: string; status: string; count: number }) {
  useCurrentLocale();
  const [attempts, setAttempts] = useState<Attempt[]>([]);
  const [next, setNext] = useState<number | null>(null);
  const [selected, setSelected] = useState('');
  const [reason, setReason] = useState('');
  const [cost, setCost] = useState('');
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const generation = useRef(0);
  const stopped = ['paused', 'interrupted', 'stopped', 'completed'].includes(status);
  useEffect(() => {
    const current = ++generation.current;
    setAttempts([]); setNext(null); setSelected(''); setConfirmed(false); setBusy(false); setMessage(''); setReason(''); setCost('');
    void invoke<Page>('get_unresolved_processing_attempts', { jobId }).then(page => {
      if (current !== generation.current) return;
      setAttempts(page.items); setNext(page.nextOffset);
    }).catch(error => { if (current === generation.current) setMessage(String(error)); });
    return () => { generation.current++; };
  }, [jobId, status, count]);
  const unknown = attempts.filter(attempt => attempt.status === 'unknown' && !attempt.remoteId);
  const validCost = cost === '' || (Number.isFinite(Number(cost)) && Number(cost) >= 0 && Number(cost) <= 10000);
  return <details className="details-collapsible-card"><Hint message={{id:"queue.unknown"}}><summary><HintTitle title={localeText("details", "ui.UnknownAttemptsPanel.d2f914e90cf18cff", {"value1": (count)})}  message={{id:"queue.unknown"}}/></summary></Hint>
    <div className="details-collapsible-card__body">
      {attempts.map(attempt => <div key={attempt.id} className="index-database-card">
        <span title={attempt.path}>{attempt.path.split(/[\\/]/).at(-1)} {' '}{localeText("details", "ui.UnknownAttemptsPanel.917a0f6dc3c3ca28")}{' '}{attempt.passNumber}</span>
        <span>{attempt.remoteId ? localeText("details", "ui.UnknownAttemptsPanel.6e539a1967e61e88", {"value1": (attempt.remoteId)}) : localeText("details", "ui.UnknownAttemptsPanel.5ebf7d619fd3da5b")}</span>
        <span>{attempt.costUsd === null ? localeText("details", "ui.UnknownAttemptsPanel.4844f87c4a20c312", {"value1": (attempt.reservedUsd.toFixed(6))}) : localeText("details", "ui.UnknownAttemptsPanel.ef415eca3ac734c1", {"value1": (attempt.costUsd.toFixed(6))})}</span>
      </div>)}
      {next !== null ? <Hint message={{id:"common.load-more",args:{список:localeText("details", "ui.UnknownAttemptsPanel.88d38e3fa2fecf1a")}}}><button type="button" className="secondary-action" disabled={busy} onClick={() => {
        const current = generation.current; setBusy(true);
        void invoke<Page>('get_unresolved_processing_attempts', { jobId, offset: next }).then(page => {
          if (current === generation.current) { setAttempts(previous => [...previous, ...page.items]); setNext(page.nextOffset); }
        }).catch(error => { if (current === generation.current) setMessage(String(error)); }).finally(() => { if (current === generation.current) setBusy(false); });
      }}>{localeText("details", "ui.UnknownAttemptsPanel.6d3e316420a72b3b")}</button></Hint> : null}
      {unknown.length ? <>
        <p>{localeText("details", "ui.UnknownAttemptsPanel.b1b82b5dfa6ec00c")}</p>
        <Hint message={{id:"queue.unknown-choice"}}><label className="settings-field"><span>{localeText("details", "ui.UnknownAttemptsPanel.06e7668fd42c2f2f")}</span><select aria-label={localeText("details", "ui.UnknownAttemptsPanel.ff89c82204daf421")} value={selected} disabled={busy || !stopped} onChange={event => { setSelected(event.target.value); setConfirmed(false); }}>
          <option value="">{localeText("details", "ui.UnknownAttemptsPanel.1031b5171fe8e04b")}</option>{unknown.map(attempt => <option key={attempt.id} value={attempt.id}>{attempt.path.split(/[\\/]/).at(-1)} {' '}{localeText("details", "ui.UnknownAttemptsPanel.917a0f6dc3c3ca28")}{' '}{attempt.passNumber} · {attempt.id.slice(-6)}</option>)}
        </select></label></Hint>
        <Hint message={{id:"queue.unknown-cost"}}><label className="settings-field"><span>{localeText("details", "ui.UnknownAttemptsPanel.283b142215ff5b7f")}</span><input aria-label={localeText("details", "ui.UnknownAttemptsPanel.4f254a90abd3ea14")} type="number" min={0} max={10000} step="0.000001" value={cost} disabled={busy} onChange={event => setCost(event.target.value)} /></label></Hint>
        <Hint message={{id:"queue.unknown-note"}}><label className="settings-field"><span>{localeText("details", "ui.UnknownAttemptsPanel.1287f5916020bb6f")}</span><textarea aria-label={localeText("details", "ui.UnknownAttemptsPanel.33edcbfca10d4749")} value={reason} disabled={busy} maxLength={1000} onChange={event => setReason(event.target.value)} /></label></Hint>
        <Hint message={{id:"queue.unknown-confirm"}}><label><input type="checkbox" checked={confirmed} disabled={busy} onChange={event => setConfirmed(event.target.checked)} />{localeText("details", "ui.UnknownAttemptsPanel.fd900efdd3842228")}</label></Hint>
        <Hint message={{id:"queue.unknown-apply"}}><button type="button" className="secondary-action" disabled={busy || !stopped || !selected || !confirmed || reason.trim().length < 10 || !validCost} onClick={() => {
          const current = generation.current; setBusy(true); setMessage('');
          void invoke('resolve_unknown_processing_attempt', { jobId, attemptId: selected, confirmed, knownCostUsd: cost === '' ? null : Number(cost), reason }).then(() => {
            if (current !== generation.current) return;
            setAttempts(previous => previous.filter(attempt => attempt.id !== selected)); setSelected(''); setConfirmed(false);
            setMessage(localeText("details", "ui.UnknownAttemptsPanel.90f2e07bbc086528")); window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));
          }).catch(error => { if (current === generation.current) setMessage(String(error)); }).finally(() => { if (current === generation.current) setBusy(false); });
        }}>{localeText("details", "ui.UnknownAttemptsPanel.550aa94b94afaa7c")}</button></Hint>
      </> : null}
      {message ? <p role="status">{message}</p> : null}
    </div>
  </details>;
}
