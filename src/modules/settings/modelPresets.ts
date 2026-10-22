import {recognitionModelLabel} from '../../shared/i18n/ModelLabels';
import {text as localeText} from "../../shared/i18n/Runtime";
import catalog from '../../shared/ai/model-catalog.json';
import type { LocalAiModelPreset, LocalAiModelPresetId, OpenRouterModelPreset, OpenRouterModelPresetId } from './Settings.types';

export const defaultLocalAiModelPresetId: LocalAiModelPresetId = 'qwen3-vl:2b-instruct-q4_K_M';

export const openRouterModelPresets: OpenRouterModelPreset[] = catalog.openrouter.map(row => ({
  id: row.id as OpenRouterModelPresetId, model: row.id, get label(){return recognitionModelLabel('openrouter',row.id,row.label);},
  priceTier: row.level as OpenRouterModelPreset['priceTier'],
  get notes(){return localeText("settings", "ui.modelPresets.09490e12e22c94d0", {"value1": (row.inputUsdPerMillion), "value2": (row.outputUsdPerMillion)});
} }));
export const falModelPresets: OpenRouterModelPreset[] = catalog.fal.map(row => ({
  id: row.id as OpenRouterModelPresetId, model: row.id, get label(){return recognitionModelLabel('fal',row.id,row.label);},
  priceTier: row.level as OpenRouterModelPreset['priceTier'],
  get notes(){return localeText("settings", "ui.modelPresets.10c32c2bc35bc54d", {"value1": (row.inputUsdPerMillion), "value2": (row.outputUsdPerMillion)});
} }));
export const localAiModelPresets: LocalAiModelPreset[] = catalog.local.map(row => ({
  id: row.id as LocalAiModelPresetId, model: row.id, get label(){return recognitionModelLabel('local',row.id,row.label);},
  get notes(){return 'fastLocal' in row&&row.fastLocal
    ? localeText("settings", "ui.modelPresets.4c358f325733a50f", {"value1": ((row.downloadBytes/1e9).toFixed(2))})
    : localeText("settings", "ui.modelPresets.e7d096cf41a68ace", {"value1": ((row.downloadBytes / 1e9).toFixed(2))});
} }));
export function resolvePresetModel(presetId: string, _customModel = '', provider: 'fal' | 'openrouter' = 'openrouter'): string {
  if (presetId === 'custom' && _customModel.trim()) return _customModel.trim();
  if (provider === 'openrouter' && presetId === 'nex-agi/nex-n2.5-mini') return presetId;
  const rows = provider === 'fal' ? falModelPresets : openRouterModelPresets;
  return rows.find(preset => preset.id === presetId)?.model ?? rows[0].id;
}
export function resolveLocalAiModel(presetId: LocalAiModelPresetId, _customModel = ''): string {
  return localAiModelPresets.find(preset => preset.id === presetId)?.model ?? defaultLocalAiModelPresetId;
}
