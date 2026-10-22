import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect, useRef, useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import { useAppState } from '../../app/state/AppStateContext';
interface Credential { provider: string; credentialRef: string; stored: boolean; persistent: boolean }
export function JobCredentialPanel({ jobId, provider, status }: { jobId: string; provider: string; status: string }) {
  useCurrentLocale();
  const { settings } = useAppState();
  const configuredReference = provider === 'fal' ? settings.fal.credentialRef : settings.openRouter.credentialRef;
  const [keys, setKeys] = useState<Credential[]>([]);
  const [selected, setSelected] = useState('');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const generation = useRef(0);
  useEffect(() => {
    const current = ++generation.current;
    setKeys([]); setSelected(''); setMessage(''); setBusy(false);
    void invoke<Credential[]>('get_provider_credentials').then(result => {
      if (generation.current !== current) return;
      setKeys(result.filter(key => key.provider === provider && key.stored));
    }).catch(error => { if (generation.current === current) setMessage(String(error)); });
    return () => { generation.current++; };
  }, [jobId, provider, status, configuredReference]);
  const allowed = ['prepared', 'paused', 'interrupted', 'stopped', 'completed'].includes(status);
  return <details className="details-collapsible-card"><Hint message={{id:"queue.job-key"}}><summary>{localeText("details", "ui.JobCredentialPanel.2156b85062ab4a55")}</summary></Hint>
    <div className="details-collapsible-card__body">
      <p>{localeText("details", "ui.JobCredentialPanel.cfad54faf561703a")}</p>
      <Hint message={{id:"queue.job-key"}}><label className="settings-field"><span>{localeText("details", "ui.JobCredentialPanel.6f4f7b74115fe41f")}</span>
        <select aria-label={localeText("details", "ui.JobCredentialPanel.1cd926ffd3bb7aac")} value={selected} disabled={busy || !allowed} onChange={event => setSelected(event.target.value)}>
          <option value="">{localeText("details", "ui.JobCredentialPanel.ed8ba4522c0e2227")}</option>
          {keys.map(key => <option key={key.credentialRef} value={key.credentialRef}>{provider === 'fal' ? 'Fal AI' : 'OpenRouter'} · {key.persistent ? localeText("details", "ui.JobCredentialPanel.58b07c12c575c14e") : localeText("details", "ui.JobCredentialPanel.73903888f62da184")} · {key.credentialRef.slice(-6)}</option>)}
        </select>
      </label></Hint>
      <Hint message={{id:"queue.job-key-apply"}}><button type="button" className="secondary-action" disabled={busy || !allowed || !selected} onClick={() => {
        const current = generation.current; setBusy(true); setMessage('');
        void invoke('set_processing_credential', { jobId, credentialRef: selected }).then(() => {
          if (current !== generation.current) return;
          setMessage(localeText("details", "ui.JobCredentialPanel.10ce6d8ca7ddc4db")); window.dispatchEvent(new CustomEvent('smart-photo-indexer:processing-wake'));
        }).catch(error => { if (current === generation.current) setMessage(String(error)); })
          .finally(() => { if (current === generation.current) setBusy(false); });
      }}>{localeText("details", "ui.JobCredentialPanel.abd7a97fb3bfd6b5")}</button></Hint>
      {!keys.length ? <p>{localeText("details", "ui.JobCredentialPanel.4c14f4018234b408")}</p> : null}
      {message ? <p role="status">{message}</p> : null}
    </div>
  </details>;
}
