import {hasVisibleDialog} from '../../shared/ui/useDialogFocus';
import {isProviderAvailable,providerUnavailableMessage} from '../../modules/settings/ProviderAvailability';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {initialQuickSearch} from '../state/QuickSearch';
import {ProcessingOptions} from '../../modules/settings/ProcessingOptions';
import {SearchFields} from '../../modules/search/SearchFields';
import {HelpButton} from '../../modules/help/HelpButton';
import {ThemeButton} from '../../shared/ui/ThemeButton';
import {Icon} from '../../shared/ui/Icon';
import {type ChangeEvent, useEffect, useRef} from 'react';
import {useFixedSelectWidth} from '../../shared/ui/useFixedSelectWidth';
import { useAppActions, useAppState } from '../state/AppStateContext';
import { selectProcessButtonLabel } from '../state/appSelectors';
import { localAiModelPresets, openRouterModelPresets, falModelPresets } from '../../modules/settings/modelPresets';
import type { LocalAiModelPresetId, OpenRouterModelPresetId } from '../../modules/settings/Settings.types';
import { ProcessButton } from '../../shared/ui/ProcessButton';
import {FolderRefreshMenu} from './FolderRefreshMenu';
import {AnalysisOutputSelect} from '../../modules/settings/AnalysisOutputSelect';
import {CompactSelect} from '../../shared/ui/CompactSelect';
import {EditSelectedButton} from '../../modules/photoEditor/EditSelectedButton';
import {openBatchRename} from '../../modules/rename/BatchRenameService';
import { messages } from '../../shared/i18n';
import {DrivePicker} from './DrivePicker';
import {useMeasuredToolbar} from '../../shared/ui/useMeasuredToolbar';

interface DisplayBreadcrumb {
  key: string;
  label: string;
  path: string;
  isCurrent: boolean;
}

function cleanBreadcrumbLabel(rawLabel: string): string {
  const label = rawLabel.replaceAll('/', '').replaceAll(String.fromCharCode(92), '').replaceAll('?', '').trim();
  if (!label) return '';
  if (/^[A-Za-z]:/.test(label)) return label.slice(0, 2);
  return label;
}

function isRootSeparatorLabel(label: string): boolean {
  const trimmed = label.trim();
  return trimmed === '/' || trimmed === String.fromCharCode(92) || trimmed === '';
}

function buildDisplayBreadcrumbs(breadcrumbs: Array<{ label: string; path: string }>, currentPath: string | null): DisplayBreadcrumb[] {
  const result: DisplayBreadcrumb[] = [];
  breadcrumbs.forEach((breadcrumb, index) => {
    const label = cleanBreadcrumbLabel(breadcrumb.label);
    if (!label) return;
    const previous = result[result.length - 1] ?? null;
    if (previous?.label.endsWith(':') && isRootSeparatorLabel(breadcrumb.label)) return;
    if (previous && previous.label === label && previous.path === breadcrumb.path) return;
    result.push({ key: `${breadcrumb.path || label}-${index}`, label, path: breadcrumb.path, isCurrent: Boolean(currentPath && breadcrumb.path === currentPath) });
  });
  return result;
}

export function MainToolbar() {
  const locale=useCurrentLocale();
  const state = useAppState();
  const actions = useAppActions();
  const quickInput = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey) || event.code !== 'KeyF' ||
          hasVisibleDialog() ||
          document.querySelector('[data-tour-id="main.search.overview"]')?.getClientRects().length) return;
      event.preventDefault();
      quickInput.current?.focus();
    };
    window.addEventListener('keydown', focusSearch, true);
    return () => window.removeEventListener('keydown', focusSearch, true);
  }, []);
  const quick=state.quickSearch??initialQuickSearch(state);
  const displayBreadcrumbs = buildDisplayBreadcrumbs(state.breadcrumbs, state.currentPath);
  const provider = state.settings.aiProviderMode;
  const presets = provider === 'local' ? localAiModelPresets : provider === 'fal' ? falModelPresets : openRouterModelPresets;
  const model = provider === 'local' ? state.settings.localAi.selectedPresetId : provider === 'fal' ? state.settings.fal.selectedPresetId : state.settings.openRouter.selectedPresetId;
  const modelLabel=presets.find(preset=>preset.id===model)?.label??model;
  const providerWidth=useFixedSelectWidth('OpenRouter'),modelWidth=useFixedSelectWidth(localeText("main", "ui.MainToolbar.8580976d41097c95"));
  const recognition=useMeasuredToolbar('data-recognition',['text','compact','icons'],locale+':'+provider+':'+modelLabel,false,'main-toolbar');
  const handleModelSelectChange = (event: ChangeEvent<HTMLSelectElement>) => {
    if (provider === 'local') actions.setLocalAiPreset(event.target.value as LocalAiModelPresetId);
    else if (provider === 'fal') actions.setFalPreset(event.target.value as OpenRouterModelPresetId);
    else actions.setOpenRouterPreset(event.target.value as OpenRouterModelPresetId);
  };

  return (
    <section className="main-toolbar" aria-label={messages().toolbar.ariaExplorerCommands}>
      <div ref={recognition.ref} data-recognition={recognition.value} className="main-toolbar__processing-row" aria-label={localeText("main", "ui.MainToolbar.257836393a9efbda")}>
        <div className="main-toolbar__folder-actions" aria-label={messages().toolbar.openFolderByPath}>
          <Hint message={{id:"main.choose-folder"}} reason={state.isScanningFolder?localeText("main", "ui.MainToolbar.2c51be8282513eb0"):undefined}><button data-tour-id="main.folder.choose" type="button" data-help-id="choose-folder" className="toolbar-choose-folder" aria-label={localeText("main", "ui.MainToolbar.d2c85ad263f173a9")} title={localeText("main", "ui.MainToolbar.0870e96ef3df1575")} disabled={state.isScanningFolder} onClick={() => void actions.chooseFolder()}><Icon name="folder"/><span>{localeText("main", "ui.MainToolbar.d3039919c2978d23")}</span></button></Hint>
          <FolderRefreshMenu/>
        </div>
        <div className="main-toolbar__processing-actions">
        <div data-help-id="recognition-model" className="main-toolbar__model-select">
          <span>{localeText("main", "ui.MainToolbar.d078d25ba2e558ad")}</span>
          <Hint message={{id:"main.provider"}}><label data-tour-id="main.recognition.provider" className="toolbar-select-control toolbar-select-control--provider" style={providerWidth.style} title={provider==='local'?localeText("main", "ui.MainToolbar.5e135fe2f84b5b69"):provider==='fal'?'Fal AI':'OpenRouter'}><Icon name="image"/><select ref={providerWidth.ref} aria-label={localeText("main", "ui.MainToolbar.29d8d1fd574e4e2c")} value={provider} style={!isProviderAvailable(provider)?{color:'var(--muted)'}:undefined} onChange={event => actions.setAiProviderMode(event.target.value as 'local' | 'openrouter' | 'fal')}>
            <option value="local">{localeText("main", "ui.MainToolbar.e65dda0098778fa3")}</option><option value="openrouter" disabled={!isProviderAvailable('openrouter')} title={!isProviderAvailable('openrouter')?providerUnavailableMessage():undefined}>{localeText("main", "ui.MainToolbar.5f1121e958c04704")}</option><option value="fal">{localeText("main", "ui.MainToolbar.aaf5c3df93ee3151")}</option>
          </select><span className="toolbar-select-chevron"><Icon name="down"/></span></label></Hint>
          <Hint message={{id:"main.model"}} reason={modelLabel}><label data-tour-id="main.recognition.model" className="toolbar-select-control toolbar-select-control--model" title={modelLabel} style={modelWidth.style}><Icon name="star"/><select ref={modelWidth.ref} aria-label={localeText("main", "ui.MainToolbar.81f39dfdb1cf7377")} value={model} disabled={!isProviderAvailable(provider)} onChange={handleModelSelectChange}>
            {presets.map(preset => <option key={preset.id} value={preset.id}>{preset.label}</option>)}
          </select><span className="toolbar-select-chevron"><Icon name="down"/></span></label></Hint>
        </div>
        <AnalysisOutputSelect compact/>
        <div data-tour-id="main.processing.overview" className="main-toolbar__launch-group"><ProcessingOptions/><ProcessButton /></div>
        <EditSelectedButton/>
        <Hint message={{id:"main.rename"}}><button data-tour-id="main.commands.rename" type="button" data-help-id="smart-rename" className="toolbar-batch-rename" aria-label={localeText("main", "ui.MainToolbar.d198dff91a1287ec")} title={localeText("main", "ui.MainToolbar.be9a3879dd39603e")} disabled={!state.selectedItemIds.length&&!state.currentPath} onClick={()=>openBatchRename()}><Icon name="tag"/></button></Hint>
        <div className="toolbar-utilities"><Hint message={{id:"main.settings"}}><button data-tour-id="main.commands.settings" type="button" data-help-id="settings" aria-label={localeText("main", "ui.MainToolbar.8fef47ca2b2608b4")} className={state.settingsPanelOpen ? 'main-toolbar__settings-button active' : 'main-toolbar__settings-button'} title={messages().toolbar.settings} onClick={actions.toggleSettingsPanel}><Icon name="settings"/><span>{localeText("main", "ui.MainToolbar.df697770c6a65967")}</span></button></Hint>
        <Hint message={{id:"main.logs"}}><button data-tour-id="main.commands.logs" type="button" aria-label={localeText("main", "ui.MainToolbar.3a754244dbf7f4d4")} className="main-toolbar__log-button" title={localeText("main", "ui.MainToolbar.fcf78ae40f04f6a7")} onClick={() => window.dispatchEvent(new CustomEvent('smart-photo-indexer:open-log-panel'))}><Icon name="file"/><span>{localeText("main", "ui.MainToolbar.cf9535b9bc61522d")}</span></button></Hint></div><div className="toolbar-appearance"><ThemeButton/></div><div className="toolbar-help"><HelpButton/></div>
        </div>
      </div>
      <div className="main-toolbar__nav-search-row" aria-label={localeText("main", "ui.MainToolbar.ca3cbbd7a53cdcd5")}>
        <div data-help-id="navigation" className="main-toolbar__navigation" aria-label={messages().toolbar.ariaNavigation}>
          <Hint message={{id:"main.back"}}><button data-tour-id="main.navigation.back" type="button" aria-label={messages().toolbar.back} title={messages().toolbar.back} disabled={state.isScanningFolder || state.navigationIndex <= 0} onClick={() => void actions.goBack()}><Icon name="left"/></button></Hint>
          <Hint message={{id:"main.forward"}}><button data-tour-id="main.navigation.forward" type="button" aria-label={messages().toolbar.forward} title={messages().toolbar.forward} disabled={state.isScanningFolder || state.navigationIndex < 0 || state.navigationIndex >= state.navigationHistory.length - 1} onClick={() => void actions.goForward()}><Icon name="right"/></button></Hint>
          <Hint message={{id:"main.up"}}><button data-tour-id="main.navigation.up" type="button" aria-label={messages().toolbar.up} title={messages().toolbar.up} disabled={state.isScanningFolder || !state.parentPath} onClick={() => void actions.goUp()}><Icon name="up"/></button></Hint>
        </div>
        <nav data-tour-id="main.navigation.breadcrumbs" className="main-toolbar__breadcrumbs" aria-label={messages().toolbar.currentFolderBreadcrumbs}>
          <DrivePicker path={state.currentPath} disabled={state.isScanningFolder} onChoose={path=>void actions.openBreadcrumb(path)}/>
          {displayBreadcrumbs.length>0?<span className="main-toolbar__breadcrumb-separator">›</span>:null}
          {displayBreadcrumbs.length > 0 ? displayBreadcrumbs.map((breadcrumb, index) => <span key={breadcrumb.key} className="main-toolbar__breadcrumb-segment">{index > 0 ? <span className="main-toolbar__breadcrumb-separator">›</span> : null}<Hint message={{id:"main.breadcrumb",args:{папка:breadcrumb.path||breadcrumb.label}}} reason={breadcrumb.path}><button data-tour-id="main.navigation.breadcrumbs" type="button" className="main-toolbar__breadcrumb-button" disabled={state.isScanningFolder || !breadcrumb.path || breadcrumb.isCurrent} title={breadcrumb.path || breadcrumb.label} onClick={() => void actions.openBreadcrumb(breadcrumb.path)}>{breadcrumb.label}</button></Hint></span>) : <span className="main-toolbar__breadcrumb-empty">{localeText("main", "ui.MainToolbar.ee386ed92550939a")}</span>}
        </nav>
        <form className="main-toolbar__search" role="search" aria-label={messages().toolbar.searchAria} onSubmit={(event) => { event.preventDefault(); actions.submitToolbarSearch(); }}>
          <div data-tour-id="main.quick.scope" data-help-id="quick-search-scope" className="toolbar-search-area" title={quick.search.rootPath?quick.search.rootPath+localeText("main", "ui.MainToolbar.100caa77c097a9c1"):localeText("main", "ui.MainToolbar.b0ade2a27108bd35")}><CompactSelect caption={localeText("main", "ui.MainToolbar.0b27cfc3947e13fd")} className="toolbar-search-scope" value={quick.search.rootPath?'folder':'library'} options={[...(state.currentPath||quick.search.rootPath?[{value:'folder',label:quick.search.rootPath&&quick.search.rootPath!==state.currentPath?localeText("main", "ui.MainToolbar.b7016f73dcd18b0b"):localeText("main", "ui.MainToolbar.9a54a5a983e86e65")}]:[]),{value:'library',label:localeText("main", "ui.MainToolbar.ddfba56be6131c57")}]} onChange={value=>actions.setQuickSearchScope('rootPath',value==='folder'?(state.currentPath??quick.search.rootPath??''):'')} message={{id:"search.scope"}}/></div>
          <SearchFields tourId="main.quick.fields" value={quick.search.fields} onChange={value=>actions.setQuickSearchScope('fields',JSON.stringify(value))}/>
          <Hint message={{id:"search.quick"}}><input ref={quickInput} data-tour-id="main.quick.query" aria-label={localeText("main", "ui.MainToolbar.10427c454bc9ff12")} onCompositionStart={()=>actions.setQuickSearchComposing(true)} onCompositionEnd={()=>actions.setQuickSearchComposing(false)} placeholder={messages().toolbar.searchPlaceholder} value={quick.query} onChange={(event) => actions.setQuickSearchQuery(event.target.value)} /></Hint>
          {quick.query.length > 0 ? <Hint message={{id:"search.quick-clear"}}><button data-tour-id="main.quick.actions" type="button" className="main-toolbar__clear-search" aria-label={messages().toolbar.clearSearch} onClick={()=>actions.clearQuickSearch()}><Icon name="close"/></button></Hint> : <kbd>{localeText("main", "ui.MainToolbar.cd84070fd036193b")}</kbd>}
          <Hint message={{id:"search.quick-submit"}}><button data-tour-id="main.quick.actions" type="submit" className="main-toolbar__search-submit" disabled={quick.search.fields?.length===0} aria-busy={quick.search.isSearching}>{quick.search.isSearching ? localeText("main", "ui.MainToolbar.6ebd8169f785596f") : localeText("main", "ui.MainToolbar.b14d971885feb911")}</button></Hint>
        </form>
      </div>
    </section>
  );
}
