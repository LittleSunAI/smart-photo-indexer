import {hasVisibleDialog} from '../../shared/ui/useDialogFocus';
import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useHelpPresentation} from '../help/HelpPresentation';
import {searchStatusHelp} from '../../shared/help/SelectionHelp';
import {Hint} from "../../shared/help/Hint";
import {SearchFields} from './SearchFields';
import {searchDraftInput,searchInputSignature} from './SearchInput';
import {useEffect,useRef,useState} from 'react';
import {Icon} from '../../shared/ui/Icon';
import type { IndexSearchStatusFilter, SearchState } from '../index/types';
import type { SearchPresetDefinition } from './SearchPresetService';
import type { SearchExportFormat, SearchExportScope } from './SearchExportService';
import type { CollectionsState } from '../collections/types';
import { messages, pluralizeRu } from '../../shared/i18n';

interface SearchControlsProps {
  currentPath?:string|null;
  onModeChange?(saved:boolean):void;
  searchQuery: string;
  recentSearches: string[];
  searchPresets: SearchPresetDefinition[];
  search: SearchState;
  collections: CollectionsState;
  selectedSearchResultCount: number;
  totalSearchResultCount: number;
  onQueryChange(query: string): void;
  onSubmit(): void;
  onLoadMore(): void;
  onClear(): void;
  onStatusFilterChange(value: IndexSearchStatusFilter): void;
  onTagFilterTextChange(value: string): void;
  onTagModeChange(value: 'any' | 'all'): void;
  onAddTagFilter(tag: string): void;
  onRemoveTagFilter(tag: string): void;
  onDateFromChange(value: string): void;
  onDateToChange(value: string): void;
  onScopeChange(field: 'fields' | 'collectionId' | 'rootPath' | 'sortOrder' | 'dateField' | 'expandTerms' | 'untaggedOnly' | 'favoriteOnly', value: string): void;
  onQuickFilter(filter: 'processedToday' | 'missing' | 'errors' | 'untagged' | 'favorites'): void;
  onSavePreset(name?: string): void;
  onApplyPreset(presetId: string): void;
  onDeletePreset(presetId: string): void;
  onTogglePresetFavorite(presetId: string): void;
  onRenamePreset(presetId: string, name: string): void;
  onExportResults(format: SearchExportFormat, scope: SearchExportScope): void;
  onCopyPaths(scope: SearchExportScope): void;
  onRevealFolders(scope: SearchExportScope): void;
  onCreateCollection(name: string): void;
  onAddSelectedToCollection(collectionId: string): void;
  onReloadCollections(): void;
  onRenameCollection(collectionId: string, name: string): void;
  onDeleteCollection(collectionId: string): void;
  onRemoveSelectedFromCollection(collectionId: string): void;
}

const statusFilters: Array<{ value: IndexSearchStatusFilter; label: string; hint: string }> = [
  { value: 'all', get label() { return messages().search.statusFilters.all.label; }, get hint() { return messages().search.statusFilters.all.hint; } },
  { value: 'processed', get label() { return messages().search.statusFilters.processed.label; }, get hint() { return messages().search.statusFilters.processed.hint; } },
  { value: 'queued', get label() { return messages().search.statusFilters.queued.label; }, get hint() { return messages().search.statusFilters.queued.hint; } },
  { value: 'notProcessed', get label() { return messages().search.statusFilters.notProcessed.label; }, get hint() { return messages().search.statusFilters.notProcessed.hint; } },
  { value: 'error', get label() { return messages().search.statusFilters.error.label; }, get hint() { return messages().search.statusFilters.error.hint; } },
  { value: 'relocated', get label() { return messages().search.statusFilters.relocated.label; }, get hint() { return messages().search.statusFilters.relocated.hint; } },
  { value: 'missing', get label() { return messages().search.statusFilters.missing.label; }, get hint() { return messages().search.statusFilters.missing.hint; } },
];

const quickFilters: Array<{ value: 'processedToday' | 'missing' | 'errors' | 'untagged' | 'favorites'; label: string }> = [
  { value: 'processedToday', get label() { return messages().search.quickFilters.processedToday; } },
  { value: 'missing', get label() { return messages().search.quickFilters.missing; } },
  { value: 'errors', get label() { return messages().search.quickFilters.errors; } },
  { value: 'favorites', get label() { return messages().search.quickFilters.favorites; } },
  { value: 'untagged', get label() { return messages().search.quickFilters.untagged; } },
];

function parseTags(value: string): string[] {
  const seen = new Set<string>();
  const tags: string[] = [];
  for (const rawTag of value.split(/[\n,;]+/)) {
    const tag = rawTag.trim().replace(/^#+/, '').toLowerCase();
    if (!tag || seen.has(tag)) continue;
    seen.add(tag);
    tags.push(tag);
  }
  return tags;
}

function formatAppliedDate(value?: number | null): string | null {
  if (!value) return null;
  return new Date(value * 1000).toLocaleDateString(displayLocaleTag());
}

export function SearchControls(p:SearchControlsProps){
  useCurrentLocale();
 const {search,searchQuery,searchPresets,collections}=p;
 const [ordinaryTab,setTab]=useState<'filters'|'saved'>('filters'),[filter,setFilter]=useState(''),[scope,setScope]=useState<SearchExportScope>('selected');
 const input=useRef<HTMLInputElement>(null),more=useRef<HTMLDialogElement>(null),actions=useRef<HTMLDialogElement>(null);
 const help=useHelpPresentation(),tab=help.view.searchPane??ordinaryTab;
 const mode=(next:'filters'|'saved')=>{setTab(next);p.onModeChange?.(next==='saved');};
 useEffect(()=>{const focus=(e:KeyboardEvent)=>{if((e.ctrlKey||e.metaKey)&&e.code==='KeyF'&&!hasVisibleDialog()&&Boolean(input.current?.closest('[data-tour-id="main.search.overview"]')?.getClientRects().length)){e.preventDefault();e.stopImmediatePropagation();setTab('filters');p.onModeChange?.(false);requestAnimationFrame(()=>input.current?.focus());}};window.addEventListener('keydown',focus,true);return()=>window.removeEventListener('keydown',focus,true);},[p.onModeChange]);
 const tags=parseTags(search.tagFilterText),applied=search.lastInput;
 const changed=Boolean(applied&&searchInputSignature(applied)!==searchInputSignature(searchDraftInput(searchQuery,search)));
 const save=()=>{const name=prompt(localeText("search", "ui.SearchControls.0502ec20c175e7e2"),buildCurrentPresetName(searchQuery,search));if(name!==null)p.onSavePreset(name);};
 return <section data-tour-id="main.search.overview" data-help-id={tab==='saved'?'search-saved':'search-filters'} className={'search-controls search-controls--compact'+(tab==='saved'?' search-controls--saved':'')} aria-label={messages().search.aria}>
  <div data-tour-id="main.search.sections" className="search-controls__heading"><div role="tablist" aria-label={localeText("search", "ui.SearchControls.1411c46c4c044b3e")}><Hint message={{id:"search.filters-tab"}}><button type="button" role="tab" aria-selected={tab==='filters'} onClick={()=>mode('filters')}>{localeText("search", "ui.SearchControls.73d3336a57b7f5bb")}</button></Hint><Hint message={{id:"search.saved-tab"}}><button type="button" role="tab" aria-selected={tab==='saved'} onClick={()=>mode('saved')}>{localeText("search", "ui.SearchControls.aeff4f93c5834296")}</button></Hint></div><Hint message={{id:"search.save"}}><button type="button" title={localeText("search", "ui.SearchControls.a448424c1c3081b7")} aria-label={localeText("search", "ui.SearchControls.3d4653dfecc7fb73")} onClick={save}><Icon name="star"/></button></Hint><Hint message={{id:"search.actions"}}><button data-tour-id="main.search.result-actions" type="button" title={localeText("search", "ui.SearchControls.1dcfff59e58edc44")} data-help-id="search-actions" aria-label={localeText("search", "ui.SearchControls.478431851a896578")} disabled={!search.queryId} onClick={()=>{setScope(p.selectedSearchResultCount?'selected':'all');actions.current?.showModal();}}><Icon name="more"/></button></Hint></div>
  <div hidden={tab!=='filters'}>
   <form data-tour-id="main.search.query" role="search" className="search-controls__query" onSubmit={e=>{e.preventDefault();p.onSubmit();}}>
    <Hint message={{id:"search.scope"}}><select data-help-id="advanced-search-scope" aria-label={localeText("search", "ui.SearchControls.6fb9b6c72285dcc6")} title={search.rootPath||localeText("search", "ui.SearchControls.abb51cc5945f0b27")} value={search.rootPath?'folder':'library'} onChange={e=>p.onScopeChange('rootPath',e.target.value==='library'?'':p.currentPath??search.rootPath)}><option value="library">{localeText("search", "ui.SearchControls.8fb39604ec6be147")}</option>{p.currentPath||search.rootPath?<option value="folder">{search.rootPath&&search.rootPath!==p.currentPath?localeText("search", "ui.SearchControls.943eaa90fa1a9a32"):localeText("search", "ui.SearchControls.cbc3d52cbc7c13c1")}</option>:null}</select></Hint>
    <SearchFields value={search.fields} onChange={value=>p.onScopeChange('fields',JSON.stringify(value))}/>
    <Hint message={{id:"search.advanced-query"}}><input ref={input} aria-label={localeText("search", "ui.SearchControls.32d8891131e9f1e5")} placeholder={localeText("search", "ui.SearchControls.47445138339ae217")} value={searchQuery} onChange={e=>p.onQueryChange(e.target.value)}/></Hint><Hint message={{id:"search.advanced-submit"}}><button type="submit" className="primary-action" disabled={search.isSearching||search.fields?.length===0}>{search.isSearching?localeText("search", "ui.SearchControls.45a44c7637902d9b"):localeText("search", "ui.SearchControls.211394001ae34409")}</button></Hint><Hint message={{id:"search.reset"}}><button type="button" aria-label={localeText("search", "ui.SearchControls.d5e51feaf3082280")} title={localeText("search", "ui.SearchControls.0ec7fd223088f691")} onClick={p.onClear}><Icon name="close"/></button></Hint><Hint message={{id:"search.extra"}}><button data-tour-id="main.search.additional" type="button" aria-label={localeText("search", "ui.SearchControls.1c80aac4d556a457")} title={localeText("search", "ui.SearchControls.d948daa04fe7b238")} onClick={()=>more.current?.showModal()}><Icon name="settings"/></button></Hint>
   </form>
   <div data-tour-id="main.search.primary-filters" className="search-controls__primary"><Hint message={{id:"search.status"}} supplement={searchStatusHelp(search.statusFilter)}><select aria-label={localeText("search", "ui.SearchControls.8e364116da6e3899")} value={search.statusFilter} onChange={e=>p.onStatusFilterChange(e.target.value as IndexSearchStatusFilter)}>{statusFilters.map(f=><option key={f.value} value={f.value}>{f.label}</option>)}</select></Hint><Hint message={{id:"search.tags"}}><input aria-label={localeText("search", "ui.SearchControls.6b44690c25021f05")} placeholder={localeText("search", "ui.SearchControls.77ac5bf2432c45bd")} value={search.tagFilterText} onChange={e=>p.onTagFilterTextChange(e.target.value)}/></Hint><Hint message={{id:search.tagMode==='all'?'search.tags-all':'search.tags-any'}}><select aria-label={localeText("search", "ui.SearchControls.1a9d4ad28480949e")} value={search.tagMode} onChange={e=>p.onTagModeChange(e.target.value as 'all'|'any')}><option value="any">{localeText("search", "ui.SearchControls.e86433004e3c35a8")}</option><option value="all">{localeText("search", "ui.SearchControls.1882179d81bf7693")}</option></select></Hint></div>
   <div className="search-controls__summary" role="status">{search.isSearching?<span>{localeText("search", "ui.SearchControls.91a7a51bad1838d9")}</span>:search.lastSearchedAtUnix?<span>{localeText("search", "ui.SearchControls.c9c836211cd04c8c")}{' '}{search.resultCount}</span>:<span>{localeText("search", "ui.SearchControls.517f6e6abaedc125")}</span>}{changed?<span>{localeText("search", "ui.SearchControls.52079e2d98451047")}</span>:null}{search.favoriteOnly?<span>{localeText("search", "ui.SearchControls.d73a61cf976d5b1b")}</span>:null}{search.untaggedOnly?<span>{localeText("search", "ui.SearchControls.369a88342f4a1988")}</span>:null}{search.expandTerms?<span>{localeText("search", "ui.SearchControls.a368c79b4239c1e2")}</span>:null}{search.collectionId?<span title={collections.items.find(c=>c.id===search.collectionId)?.name}>{localeText("search", "ui.SearchControls.40ed34e9bc15799f")}</span>:null}{search.dateFrom||search.dateTo?<span title={search.dateFrom+' — '+search.dateTo}>{localeText("search", "ui.SearchControls.b09de19ff7e1af3c")}{' '}{search.dateField==='taken'?localeText("search", "ui.SearchControls.c6e57360082a27f4"):search.dateField==='indexed'?localeText("search", "ui.SearchControls.6172254d4f4b1f79"):search.dateField==='analyzed'?localeText("search", "ui.SearchControls.21d35bb92cd2fb7d"):localeText("search", "ui.SearchControls.528967f4bd8610f8")}</span>:null}{search.untaggedOnly&&tags.length?<span className="search-controls__error">{localeText("search", "ui.SearchControls.67f1719210f52716")}</span>:null}{search.errorMessage?<span role="alert" className="search-controls__error">{nativeDisplayText(search.errorMessage??'')}</span>:null}</div>
  </div>
  <div data-tour-id="main.search.saved" hidden={tab!=='saved'} className="search-presets-panel"><Hint message={{id:"search.saved-filter"}}><input aria-label={localeText("search", "ui.SearchControls.a5929ab5b4d8d0da")} placeholder={localeText("search", "ui.SearchControls.d8595b8bcbbf4517")} value={filter} onChange={e=>setFilter(e.target.value)}/></Hint><div className="search-presets-list">{searchPresets.filter(preset=>preset.name.toLocaleLowerCase().includes(filter.toLocaleLowerCase())).map(preset=><article key={preset.id} className="search-preset-card"><Hint message={{id:"search.saved-open",args:{название:preset.name}}}><button type="button" className="search-preset-card__main" title={describePreset(preset)} onClick={()=>{p.onApplyPreset(preset.id);mode('filters');}}><strong>{preset.isFavorite?<Icon name="star" filled/>:null}{preset.name}</strong><span>{describePreset(preset)}</span></button></Hint><div className="search-preset-card__actions"><Hint message={{id:"search.saved-star"}}><button type="button" aria-label={localeText("search", "ui.SearchControls.ed4f9dd824c5f39b")+preset.name} onClick={()=>p.onTogglePresetFavorite(preset.id)}><Icon name="star" filled={preset.isFavorite}/></button></Hint><Hint message={{id:"search.saved-rename"}}><button type="button" aria-label={localeText("search", "ui.SearchControls.50d73f688f4eaf2b")+preset.name} onClick={()=>{const name=prompt(localeText("search", "ui.SearchControls.bf58fd05f59a762c"),preset.name);if(name!==null)p.onRenamePreset(preset.id,name);}}><Icon name="edit"/></button></Hint><Hint message={{id:"search.saved-remove"}}><button type="button" aria-label={localeText("search", "ui.SearchControls.27bb2bc7095cae97")+preset.name} onClick={()=>p.onDeletePreset(preset.id)}><Icon name="close"/></button></Hint></div></article>)}</div>{!searchPresets.length?<p>{localeText("search", "ui.SearchControls.1f3cf9ebf26b5d49")}</p>:null}{p.recentSearches.length?<details><Hint message={{id:"common.expand",args:{раздел:localeText("search", "ui.SearchControls.67870fe5412053b9")}}}><summary>{localeText("search", "ui.SearchControls.b86a80c1ea440310")}</summary></Hint>{p.recentSearches.map(query=><Hint key={query} message={{id:"search.recent"}}><button type="button" key={query} onClick={()=>{p.onQueryChange(query);mode('filters');}}>{query}</button></Hint>)}</details>:null}</div>
  {search.operationFeedback?<p className={"search-operation-feedback"+(search.operationFeedback.error?" danger":"")} role={search.operationFeedback.error?"alert":"status"}>{search.operationFeedback.message}</p>:null}
  <dialog ref={more} className="search-options-dialog" aria-labelledby="search-extra-title"><header><strong id="search-extra-title">{localeText("search", "ui.SearchControls.64a5236a43792acf")}</strong><Hint message={{id:"common.close-dialog"}}><button type="button" aria-label={localeText("search", "ui.SearchControls.980088884cf1311a")} onClick={()=>more.current?.close()}><Icon name="close"/></button></Hint></header><div className="search-options-dialog__fields">
   {(['favoriteOnly','untaggedOnly','expandTerms'] as const).map((field,index)=><Hint key={field} message={{id:({favoriteOnly:"search.favorite",untaggedOnly:"search.condition-untagged",expandTerms:"search.word-forms"} as const)[field]}}><label className="search-options-dialog__check" key={field}><input type="checkbox" checked={Boolean(search[field])} onChange={e=>p.onScopeChange(field,String(e.target.checked))}/>{[localeText("search", "ui.SearchControls.535d04aff04d75dc"),localeText("search", "ui.SearchControls.369a88342f4a1988"),localeText("search", "ui.SearchControls.b7f6db880a13cf73")][index]}</label></Hint>)}
   <Hint message={{id:"search.collection"}}><label>{localeText("search", "ui.SearchControls.40ed34e9bc15799f")}<select aria-label={localeText("search", "ui.SearchControls.7ca4c4a644f720d6")} value={search.collectionId} onChange={e=>p.onScopeChange('collectionId',e.target.value)}><option value="">{localeText("search", "ui.SearchControls.63381f2b2d5d2098")}</option>{collections.items.map(c=><option key={c.id} value={c.id}>{c.name}</option>)}</select></label></Hint><Hint message={{id:"search.root"}}><label>{localeText("search", "ui.SearchControls.11b39a0213415b8e")}<input aria-label={localeText("search", "ui.SearchControls.186cc3c6fc32a857")} value={search.rootPath} placeholder={localeText("search", "ui.SearchControls.409c8f1d76abd323")} onChange={e=>p.onScopeChange('rootPath',e.target.value)}/></label></Hint>
   <Hint message={{id:"search.date-kind"}}><label>{localeText("search", "ui.SearchControls.b6493b6729e8154b")}<select aria-label={localeText("search", "ui.SearchControls.cee9f5469e72772d")} value={search.dateField} onChange={e=>p.onScopeChange('dateField',e.target.value)}><option value="modified">{localeText("search", "ui.SearchControls.de4622545714d9bc")}</option><option value="taken">{localeText("search", "ui.SearchControls.73d27d24972205f2")}</option><option value="indexed">{localeText("search", "ui.SearchControls.74451045bcae7cbc")}</option><option value="analyzed">{localeText("search", "ui.SearchControls.6297bd8a31dd8450")}</option></select></label></Hint><Hint message={{id:"search.date-from"}}><label>{localeText("search", "ui.SearchControls.31161802ea87f3d5")}<input aria-label={localeText("search", "ui.SearchControls.5f30a6c1b1ce3c90")} type="date" value={search.dateFrom} onChange={e=>p.onDateFromChange(e.target.value)}/></label></Hint><Hint message={{id:"search.date-to"}}><label>{localeText("search", "ui.SearchControls.445ea87a3d7efdda")}<input aria-label={localeText("search", "ui.SearchControls.ba3b288b9298c86f")} type="date" value={search.dateTo} onChange={e=>p.onDateToChange(e.target.value)}/></label></Hint><details><Hint message={{id:"common.expand",args:{раздел:localeText("search", "ui.SearchControls.82bf55e1641c9426")}}}><summary>{localeText("search", "ui.SearchControls.407aa64cfe23cf9e")}</summary></Hint>{quickFilters.map(f=><Hint key={f.value} message={{id:({processedToday:'search.condition-today',missing:'search.condition-missing',errors:'search.condition-errors',favorites:'search.condition-favorites',untagged:'search.condition-untagged'} as const)[f.value]}}><button type="button" key={f.value} onClick={()=>p.onQuickFilter(f.value)}>{f.label}</button></Hint>)}</details></div><footer><Hint message={{id:"search.conditions-done"}}><button type="button" className="primary-action" onClick={()=>more.current?.close()}>{localeText("search", "ui.SearchControls.18eee2a7023642cf")}</button></Hint></footer></dialog>
  <dialog ref={actions} className="search-options-dialog" aria-labelledby="search-actions-title"><header><strong id="search-actions-title">{localeText("search", "ui.SearchControls.f3a1ee1d8106a36b")}</strong><Hint message={{id:"common.close-dialog"}}><button type="button" aria-label={localeText("search", "ui.SearchControls.8dbb6eae6e501f9f")} onClick={()=>actions.current?.close()}><Icon name="close"/></button></Hint></header><Hint message={{id:"search.action-scope"}}><label>{localeText("search", "ui.SearchControls.5b663481197730cf")}<select aria-label={localeText("search", "ui.SearchControls.d99a412a25997df9")} value={scope} onChange={e=>setScope(e.target.value as SearchExportScope)}><option value="selected" disabled={!p.selectedSearchResultCount}>{localeText("search", "ui.SearchControls.7880395c71d20c03")}{p.selectedSearchResultCount})</option><option value="all">{localeText("search", "ui.SearchControls.4f469ad86dc53c6f")}{search.resultCount})</option></select></label></Hint><div className="search-options-dialog__actions"><Hint message={{id:"search.csv"}}><button type="button" onClick={()=>{p.onExportResults('csv',scope);actions.current?.close();}}>{localeText("search", "ui.SearchControls.b5af6b5155ec3fa0")}</button></Hint><Hint message={{id:"search.json"}}><button type="button" onClick={()=>{p.onExportResults('json',scope);actions.current?.close();}}>{localeText("search", "ui.SearchControls.824edc70534c3789")}</button></Hint><Hint message={{id:"search.paths"}}><button type="button" onClick={()=>{p.onCopyPaths(scope);actions.current?.close();}}>{localeText("search", "ui.SearchControls.5c320dd1d648720f")}</button></Hint><Hint message={{id:"search.folders"}}><button type="button" title={localeText("search", "ui.SearchControls.6cb17cec1622d715")} onClick={()=>{p.onRevealFolders(scope);actions.current?.close();}}>{localeText("search", "ui.SearchControls.45a29f5d4f12925f")}</button></Hint></div></dialog>
 </section>;
}

function buildCurrentPresetName(query: string, search: SearchState): string {
  const parts: string[] = [];
  const cleanQuery = query.trim();
  const tags = parseTags(search.tagFilterText);
  if (cleanQuery) parts.push(cleanQuery.length > 36 ? `${cleanQuery.slice(0, 33)}…` : cleanQuery);
  if (tags.length > 0) parts.push(tags.slice(0, 3).map((tag) => `#${tag}`).join(' '));
  if (search.statusFilter !== 'all') parts.push(`${messages().search.describe.status}: ${search.statusFilter}`);
  if (search.dateFrom || search.dateTo) parts.push(`${messages().search.describe.date}: ${search.dateFrom || localeText("search", "ui.SearchControls.b79b875bf6e59338")}–${search.dateTo || localeText("search", "ui.SearchControls.b79b875bf6e59338")}`);
  if (search.favoriteOnly) parts.push(messages().search.describe.favoritesOnly);
  if (search.untaggedOnly) parts.push(localeText("search", "ui.SearchControls.17fd479020d46b17"));
  return parts.length > 0 ? parts.join(' · ') : messages().search.buildPresetName;
}

function describePreset(preset: SearchPresetDefinition): string {
  const parts: string[] = [];
  if (preset.query.trim()) parts.push(`${messages().search.describe.query}: ${preset.query.trim()}`);
  if (preset.statusFilter !== 'all') parts.push(`${messages().search.describe.status}: ${preset.statusFilter}`);
  const tags = parseTags(preset.tagFilterText);
  if (tags.length > 0) parts.push(`${messages().search.describe.tags} ${preset.tagMode === 'all' ? messages().search.describe.tagModeAll : messages().search.describe.tagModeAny}: ${tags.map((tag) => `#${tag}`).join(' ')}`);
  if (preset.dateFrom || preset.dateTo) parts.push(`${messages().search.describe.date}: ${preset.dateFrom || localeText("search", "ui.SearchControls.82a680b3a822bb19")} – ${preset.dateTo || localeText("search", "ui.SearchControls.82a680b3a822bb19")}`);
  if (preset.favoriteOnly) parts.push(messages().search.describe.favoritesOnly);
  if (preset.untaggedOnly) parts.push(localeText("search", "ui.SearchControls.d5b61f78a0bd7567"));
  if (preset.lastUsedAtUnix) parts.push(`${messages().search.describe.used}: ${new Date(preset.lastUsedAtUnix * 1000).toLocaleDateString(displayLocaleTag())}`);
  return parts.length > 0 ? parts.join(' · ') : messages().search.emptyPreset;
}
