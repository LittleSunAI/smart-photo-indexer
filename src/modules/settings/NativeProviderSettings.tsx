import {nativeDisplayText,uiFeedbackText} from '../../shared/i18n/NativeMessages';
import {isProviderAvailable,providerUnavailableMessage} from './ProviderAvailability';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect, useRef, useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import { useAppActions, useAppDispatch, useAppState } from '../../app/state/AppStateContext';
import { localAiModelPresets, openRouterModelPresets, falModelPresets } from './modelPresets';
import type { AiProviderMode, LocalAiModelPresetId, OpenRouterModelPresetId } from './Settings.types';
import { ProviderCredentialFields } from './ProviderCredentialFields';
import {AnalysisOutputSelect} from './AnalysisOutputSelect';
import {HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';
import { cancelLocalInstallation, checkLocalInstallation, installLocalModel, useLocalInstallation } from '../localAi/LocalModelInstallation';

export function NativeProviderSettings({active=true}:{active?:boolean}) {
  useCurrentLocale();
  const { settings } = useAppState();
  const actions = useAppActions();
  const dispatch = useAppDispatch();
  const provider = settings.aiProviderMode;
  const cloud = provider === 'fal' ? settings.fal : settings.openRouter;
  const model = provider === 'local' ? settings.localAi.selectedModel : cloud.selectedModel;
  const presets = provider === 'local' ? localAiModelPresets : provider === 'fal' ? falModelPresets : openRouterModelPresets;
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState('');
  const [keyDrafts, setKeyDrafts] = useState({ fal: '', openrouter: '' });
  const generation = useRef(0);
  const local = useLocalInstallation(settings.localAi.selectedModel, settings.localAi.endpoint, provider === 'local'&&active);
  useEffect(() => { generation.current++; setMessage(''); setBusy(false); }, [provider, model]);

  async function check() {
    if (provider === 'local') { await checkLocalInstallation(model, settings.localAi.endpoint); return; }
    const current = generation.current;
    setBusy(true);
    try {
      const result = await invoke<{ message: string }>('get_ai_provider_status', { provider, model, endpoint: settings.localAi.endpoint, credentialRef: cloud.credentialRef });
      if (current === generation.current) setMessage(result.message);
    } catch (error) { if (current === generation.current) setMessage(String(error)); }
    finally { if (current === generation.current) setBusy(false); }
  }
  async function download() {
    await installLocalModel(model, settings.localAi.endpoint);
  }

  return <section className="settings-card">
    <h3><HintTitle title={localeText("settings", "ui.NativeProviderSettings.d53f302b7d1890fe")}  message={{id:"main.provider"}}/></h3>
    <Hint message={{id:"main.provider"}}><label className="settings-field"><span>{localeText("settings", "ui.NativeProviderSettings.93df65562f16dab9")}</span>
      <select data-tour-id="settings.recognition.source" aria-label={localeText("settings", "ui.NativeProviderSettings.74d618a9b15e6517")} value={provider} style={!isProviderAvailable(provider)?{color:'var(--muted)'}:undefined} onChange={event => actions.setAiProviderMode(event.target.value as AiProviderMode)}>
        <option value="local">{localeText("settings", "ui.NativeProviderSettings.6cee5cf8ef979949")}</option><option value="openrouter" disabled={!isProviderAvailable('openrouter')} title={!isProviderAvailable('openrouter')?providerUnavailableMessage():undefined}>{localeText("settings", "ui.NativeProviderSettings.cca277b68cdf7cff")}</option><option value="fal">{localeText("settings", "ui.NativeProviderSettings.aace7597812506a8")}</option>
      </select>
    </label></Hint>
    <Hint message={{id:"main.model"}}><label className="settings-field"><span>{localeText("settings", "ui.NativeProviderSettings.4d0b63fdfee6da9e")}</span>
      <select data-tour-id="settings.recognition.model" aria-label={localeText("settings", "ui.NativeProviderSettings.8a7a1d45ead4db8c")} value={model} disabled={!isProviderAvailable(provider)} onChange={event => {
        if (provider === 'local') actions.setLocalAiPreset(event.target.value as LocalAiModelPresetId);
        else if (provider === 'fal') actions.setFalPreset(event.target.value as OpenRouterModelPresetId);
        else actions.setOpenRouterPreset(event.target.value as OpenRouterModelPresetId);
      }}>{presets.map(preset => <option key={preset.id} value={preset.id}>{preset.label}</option>)}</select>
    </label></Hint>
    <p className="settings-muted">{presets.find(preset => preset.id === model)?.notes}</p>
    <AnalysisOutputSelect/>
    {provider === 'local' ? <>
      <Hint message={{id:"local.endpoint"}}><label className="settings-field"><span>{localeText("settings", "ui.NativeProviderSettings.55ea3dd87abd6372")}</span><input value={settings.localAi.endpoint} onChange={event => actions.setLocalAiEndpoint(event.target.value)} spellCheck={false} /></label></Hint>
      <div className="settings-action-row">
        <Hint message={{id:"local.check"}}><button type="button" disabled={local.state==='checking'||local.state==='installing'} onClick={() => void check()}>{localeText("settings", "ui.NativeProviderSettings.787f3fd1b5c15912")}</button></Hint>
        <Hint message={{id:local.state==='ready'?'local.ready':local.state==='installing'?'local.installing':'local.setup'}}><button type="button" disabled={['checking','installing','ready'].includes(local.state)} onClick={() => void download()}>{local.state==='ready'?localeText("settings", "ui.NativeProviderSettings.afa8b8a6928ac036"):local.state==='installing'?localeText("settings", "ui.NativeProviderSettings.3bddc17ba52b2b78", {"value1": (local.percent===undefined?'…':` ${local.percent}%`)}):localeText("settings", "ui.NativeProviderSettings.96c8c8e133069624")}</button></Hint>
        {local.state==='installing'&&local.cancellable?<Hint message={{id:"local.cancel-install"}}><button type="button" onClick={()=>void cancelLocalInstallation().catch(error=>setMessage(String(error)))}>{localeText("settings", "ui.NativeProviderSettings.7298a8e1307086df")}</button></Hint>:null}
      </div>
      <p className="settings-muted">{localeText("settings", "ui.NativeProviderSettings.52c0bb0d22282d3c")}</p>
      <p className="settings-check" role="status">{local.messageKey?localeText('settings',local.messageKey):nativeDisplayText(local.message)}{local.percent===undefined?'':` ${local.percent}%`}</p>
    </> : null}
    {message ? <p className="settings-check" role="status">{uiFeedbackText('settings',message)}</p> : null}
    {provider !== 'local' ? <ProviderCredentialFields key={provider} provider={provider} model={model} credentialRef={cloud.credentialRef}
      draft={keyDrafts[provider]} onDraftChange={value => setKeyDrafts(previous => ({ ...previous, [provider]: value }))}/> : null}
  </section>;
}
