import{requestSettingsTab}from'../../modules/settings/SettingsNavigation';
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useHelpPresentation} from '../../modules/help/HelpPresentation';
import {workspacePresentation} from './WorkspacePresentation';
import type {TourPresentation} from '../../modules/help/HelpPresentation';
import type {AppState} from '../types';
import {Hint} from "../../shared/help/Hint";
import {isQueryView} from '../state/QuickSearch';
import {FilenameFilter} from '../../modules/explorer/FilenameFilter';
import {useUiPreferences} from '../../shared/useUiPreferences';
import {saveUiPreferences} from '../../shared/UiPreferences';
import {ExplorerSortSelect} from '../../modules/explorer/ExplorerSortSelect';
import { memo, useCallback, useEffect, useMemo, useRef, useState, type PointerEvent, type KeyboardEvent } from 'react';
import { ExplorerGrid } from '../../modules/explorer/ExplorerGrid';
import {InfoHint} from '../../shared/ui/InfoHint';
import { ExplorerContextMenu } from '../../modules/explorer/ExplorerContextMenu';
import { DetailsPanel } from '../../modules/details/DetailsPanel';
import { SearchControls } from '../../modules/search/SearchControls';
import { useAppActions, useAppSelector } from '../state/AppStateContext';
import {
  selectContextMenuItem,
  selectSelectedImageItems,
  selectSelectedItem,
  selectSelectedItems,
  selectWorkspaceItems,
  selectViewerImages,
} from '../state/appSelectors';
import { openSmartImageViewer } from '../../modules/imageViewer';
import type { ExplorerItem } from '../../modules/explorer/types';
import type { SortMode, ViewSize } from '../types';
import { messages } from '../../shared/i18n';
import { loadWorkspacePreferences } from '../../modules/settings/WorkspacePreferences';
import {searchOrderFor,sortForSearchOrder} from '../../modules/explorer/ExplorerViews';
import { CompactSelect } from '../../shared/ui/CompactSelect';
import { Icon } from '../../shared/ui/Icon';
import {ImageComparison,comparisonImage,type ComparisonImage} from '../../modules/details/ImageComparison';
import {CollectionsWorkspace} from '../../modules/collections/CollectionsWorkspace';
import {useDialogFocus} from '../../shared/ui/useDialogFocus';
import {useMeasuredToolbar} from '../../shared/ui/useMeasuredToolbar';

function sameStringArray(left: string[], right: string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function sameItemArray<T extends ExplorerItem>(left: T[], right: T[]): boolean {
  return left.length === right.length && left.every((item, index) => item === right[index]);
}
const MIN_DETAILS_WIDTH = 360;
// Six workspace tabs and all five compact view controls must remain usable
// together with comparison and the filename filter in a single header row.
const MIN_MAIN_WIDTH = 480;
const SPLITTER_WIDTH = 6;

export function WorkspaceLayout() {
  useCurrentLocale();
  const help=useHelpPresentation();
  const preview=help.view.workspace||help.view.recent?help.view:undefined;
  const helpDetails=Boolean(help.view.rightPanel);
  const [narrow,setNarrow]=useState(()=>window.innerWidth<=920),[detailsOpen,setDetailsOpen]=useState(false);
  const toggleDetails=useCallback(()=>setDetailsOpen(value=>!value),[]);
  const detailsHost=useRef<HTMLDivElement>(null);
  useEffect(()=>{const update=()=>setNarrow(window.innerWidth<=920);window.addEventListener('resize',update);return()=>window.removeEventListener('resize',update);},[]);
  const layoutRef = useRef<HTMLElement>(null);
  const dragRef = useRef<{ x: number; width: number } | null>(null);
  const [panelWidth, setPanelWidth] = useState(() => {
    try {
      const saved = Number(localStorage.getItem('smart-photo:right-panel-width'));
      return Number.isFinite(saved) && saved >= MIN_DETAILS_WIDTH ? saved : MIN_DETAILS_WIDTH;
    } catch { return 360; }
  });
  const bounds = () => {
    const node = layoutRef.current;
    const padding = node ? parseFloat(getComputedStyle(node).paddingLeft) + parseFloat(getComputedStyle(node).paddingRight) : 16;
    const available = Math.max(0, (node?.clientWidth ?? 1000) - padding);
    const min = MIN_DETAILS_WIDTH;
    return { min, max: Math.max(min, Math.min(600, available - MIN_MAIN_WIDTH - SPLITTER_WIDTH)) };
  };
  const resize = (width: number) => {
    const { min, max } = bounds();
    setPanelWidth(Math.round(Math.max(min, Math.min(max, width))));
    window.dispatchEvent(new Event('smart-photo-indexer:panel-width-intent'));
  };
  useEffect(() => {
    try { localStorage.setItem('smart-photo:right-panel-width', String(panelWidth)); } catch { /* Layout remains usable without storage. */ }
  }, [panelWidth]);
  useEffect(() => {
    const element = layoutRef.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setPanelWidth((width) => {
      if(window.innerWidth<=920)return width;
      const { min, max } = bounds();
      return Math.round(Math.max(min, Math.min(max, width)));
    }));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  const move = (event: PointerEvent<HTMLDivElement>) => {
    if (dragRef.current) resize(dragRef.current.width + dragRef.current.x - event.clientX);
  };
  const end = () => { dragRef.current = null; };
  const key = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const { min, max } = bounds();
    resize(event.key === 'Home' ? min : event.key === 'End' ? max : panelWidth + (event.key === 'ArrowLeft' ? 1 : -1) * (event.shiftKey ? 40 : 10));
  };
  return (
    <main ref={layoutRef} className="workspace-layout" style={{ gridTemplateColumns: `minmax(0, 1fr) 6px ${panelWidth}px` }}>
      <MemoMainPane hidden={Boolean(preview)} narrow={narrow} detailsOpen={detailsOpen||helpDetails} onToggleDetails={toggleDetails}/>
      {preview?<MemoMainPane preview={preview} narrow={narrow} detailsOpen={detailsOpen||helpDetails} onToggleDetails={toggleDetails}/>:null}
      <Hint message={{id:"layout.panel-width"}}><div className="workspace-layout__splitter" role="separator" aria-label={localeText("main", "ui.WorkspaceLayout.3d2ec733349c6d9e")} aria-orientation="vertical"
        aria-valuenow={panelWidth} aria-valuemin={Math.round(bounds().min)} aria-valuemax={Math.round(bounds().max)} tabIndex={0}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          event.currentTarget.setPointerCapture(event.pointerId);
          dragRef.current = { x: event.clientX, width: panelWidth };
        }} onPointerMove={move} onPointerUp={end} onPointerCancel={end} onLostPointerCapture={end} onKeyDown={key}><span /></div></Hint>
      {narrow&&detailsOpen?<Hint message={{id:"main.close-panel"}}><button type="button" className="workspace-details-scrim" aria-label={localeText("main", "ui.WorkspaceLayout.21f3f7c4489c7c64")} onClick={()=>setDetailsOpen(false)}/></Hint>:null}
      <div ref={detailsHost} className={'workspace-details-host'+(narrow?' is-narrow':'')+(detailsOpen||helpDetails?' is-open':'')} role={narrow&&detailsOpen&&!help.open?'dialog':undefined} aria-modal={narrow&&detailsOpen&&!help.open?true:undefined} aria-label={narrow&&detailsOpen?localeText("main", "ui.WorkspaceLayout.da6e18e80dc21f39"):undefined}>
        {narrow&&detailsOpen?<><div className="workspace-details-heading"><strong>{localeText("main", "ui.WorkspaceLayout.d63a4e863536c86a")}</strong><Hint message={{id:"main.close-panel"}}><button type="button" aria-label={localeText("main", "ui.WorkspaceLayout.3e50b238642d53fa")} onClick={()=>setDetailsOpen(false)}><Icon name="close"/></button></Hint></div>{!help.open?<NarrowDetailsFocus host={detailsHost} onClose={()=>setDetailsOpen(false)}/>:null}</>:null}
        <MemoDetailsPane />
      </div>
    </main>
  );
}
function NarrowDetailsFocus({host,onClose}:{host:React.RefObject<HTMLElement|null>;onClose():void}){useDialogFocus(host,onClose);return null;}

function WorkspaceMainPane({narrow,detailsOpen,onToggleDetails,hidden=false,preview}:{narrow:boolean;detailsOpen:boolean;onToggleDetails():void;hidden?:boolean;preview?:TourPresentation}) {
  const locale=useCurrentLocale();
  const help=useHelpPresentation();
  function useViewSelector<T>(selector:(state:AppState)=>T,equal?:(left:T,right:T)=>boolean){return useAppSelector(state=>selector(preview?workspacePresentation(state,preview):state),equal);}
  const [ordinaryRecentOpen,setRecentOpen]=useState(false);
  const recentOpen=preview?Boolean(preview.recent):ordinaryRecentOpen;
  const [ordinarySavedSearchOpen,setSavedSearchOpen]=useState(false);
  const savedSearchOpen=preview?preview.searchPane==='saved':ordinarySavedSearchOpen;

  const uiPreferences=useUiPreferences();
  const showImagesOnly=useViewSelector(state=>state.showImagesOnly);
  const [comparison,setComparison]=useState<[ComparisonImage,ComparisonImage]|null>(null);
  const [comparisonError,setComparisonError]=useState('');
  const [comparisonBusy,setComparisonBusy]=useState(false);
  const actions = useAppActions();
  const activeWorkspace = useViewSelector((state) => state.activeWorkspace);
  const quickActive=useViewSelector(s=>Boolean(s.quickSearch?.active));
  const queryWorkspace=useViewSelector(isQueryView);
  const underlying=useViewSelector(s=>s.underlyingSearch);
  const currentPath=useViewSelector(state=>state.currentPath);
  const recentPaths=loadWorkspacePreferences().recentPaths;
  const viewSize = useViewSelector((state) => state.viewSize);
  const filenameFilter=useViewSelector(state=>state.filenameFilter??'');
  const sortMode = useViewSelector((state) => state.sortMode);
  const selectedItemId = useViewSelector((state) => state.selectedItemId);
  const selectedItemIds = useViewSelector((state) => state.selectedItemIds, sameStringArray);
  const searchQuery = useViewSelector((state) => state.searchQuery);
  const recentSearches = useViewSelector((state) => state.recentSearches, sameStringArray);
  const searchPresets = useViewSelector((state) => state.searchPresets);
  const search = useViewSelector((state) => state.search);
  const collections = useViewSelector((state) => state.collections);
  const showingCollection=activeWorkspace==='collections'&&collections.items.some(item=>item.id===search.collectionId);
  const isScanningFolder = useViewSelector((state) => state.isScanningFolder);
  const isFolderScanActive=useViewSelector(state=>state.isFolderScanActive);
  const filesystemError = useViewSelector((state) => state.filesystemError);
  const contextMenu = useViewSelector((state) => state.contextMenu);
  const workspaceItems = useViewSelector(selectWorkspaceItems, sameItemArray);
  const viewerImages = useViewSelector(selectViewerImages, sameItemArray);
  const selectedItems = useViewSelector(selectSelectedItems, sameItemArray);
  const selectedImageItems = useViewSelector(selectSelectedImageItems, sameItemArray);
  const contextMenuItem = useViewSelector(selectContextMenuItem);
  const visibleItemIds = workspaceItems.map((item) => item.id);
   const applySort=(sort:SortMode)=>{if(queryWorkspace){const order=searchOrderFor(sort);if(quickActive)actions.setQuickSearchScope('sortOrder',order);else actions.setSearchScope('sortOrder',order);actions.submitSearch(order);}else actions.setSortMode(sort);};
  const appliedOrder=search.appliedSortOrder??'relevance';
  const activeSort=queryWorkspace?sortForSearchOrder(appliedOrder):sortMode;
  const selectedSearchResultCount = useViewSelector(
    (state) => state.search.selectionSnapshot
      ? state.search.selectionSnapshot.totalCount-state.search.selectionSnapshot.excludedPaths.length
      : state.search.results.filter((item) => state.selectedItemIds.includes(item.id)).length,
  );
  const comparisonCount=queryWorkspace?selectedSearchResultCount:selectedImageItems.length;
  const header=useMeasuredToolbar('data-header-mode',['full','values','right-icons','icons'],[locale,viewSize,activeSort,showImagesOnly,comparisonCount,recentOpen,queryWorkspace,activeWorkspace].join(':'),true);
  const compare=async()=>{
    if(comparisonBusy||comparisonCount!==2)return;
    setComparisonBusy(true);setComparisonError('');actions.closeContextMenu();
    try{const images=queryWorkspace?await actions.resolveSelectedImages(3):selectedImageItems;const unique=[...new Map(images.map(item=>[item.path,item])).values()];
      if(unique.length!==2)throw new Error(localeText("main", "ui.WorkspaceLayout.8523ae8a025a147f"));
      setComparison([comparisonImage(unique[0]),comparisonImage(unique[1])]);
    }catch(error){setComparisonError(String(error));}finally{setComparisonBusy(false);}
  };

  const openWorkspaceItem = (itemId: string) => {
    const item = workspaceItems.find((candidate) => candidate.id === itemId);
    if (item?.kind === 'image') {
      if(queryWorkspace && search.queryId) {void actions.openSearchViewer(itemId);return;}
      void openSmartImageViewer(item, viewerImages);
      return;
    }

    actions.openSelectedFile(itemId);
  };

  return (
    <section hidden={hidden} data-tour-preview={preview?true:undefined} className={`workspace-layout__main-pane${activeWorkspace === 'search' ? ' workspace-layout__main-pane--search' : ''}`} aria-label={localeText("main", "ui.WorkspaceLayout.3ad2db35aba99614")}>
      <div ref={header.ref} data-header-mode={header.value} data-compact-level={header.level<2?0:header.level===2?1:2} data-help-id="workspace-tabs" className="workspace-layout__tabs" role="tablist" aria-label={messages().workspace.mode}>
        <Hint message={{id:"main.tab-explorer"}}><button data-tour-id="main.workspace.explorer" type="button" className={!recentOpen&&activeWorkspace === 'explorer' ? 'active' : undefined} role="tab" aria-label={messages().workspace.explorerTab} title={messages().workspace.explorerTab} aria-selected={!recentOpen&&activeWorkspace === 'explorer'} onClick={() => {setRecentOpen(false);actions.setWorkspaceMode('explorer');}}>
          <Icon name="folder"/><span>{messages().workspace.explorerTab}</span>
        </button></Hint>
        <Hint message={{id:"main.tab-search"}}><button data-tour-id="main.workspace.search-tab" type="button" className={!recentOpen&&activeWorkspace === 'search' ? 'active' : undefined} role="tab" aria-label={messages().workspace.searchTab} title={messages().workspace.searchTab} aria-selected={!recentOpen&&activeWorkspace === 'search'} onClick={() => {setRecentOpen(false);actions.setWorkspaceMode('search');}}>
          <Icon name="search"/><span>{messages().workspace.searchTab}</span>
        </button></Hint>
        {(['favorites','ready','collections'] as const).map(mode=><Hint key={mode} message={{id:({favorites:'main.tab-favorites',ready:'main.tab-ready',collections:'main.tab-collections'} as const)[mode]}}><button data-tour-id={'main.workspace.'+mode} key={mode} type="button" role="tab" className={!recentOpen&&activeWorkspace===mode?'active':undefined} aria-selected={!recentOpen&&activeWorkspace===mode}
          onClick={()=>{setRecentOpen(false);actions.setWorkspaceMode(mode);}} title={mode==='favorites'?localeText("main", "ui.WorkspaceLayout.8c0178ae414e1ed1"):mode==='ready'?localeText("main", "ui.WorkspaceLayout.662409235b1a5bb3"):localeText("main", "ui.WorkspaceLayout.8f128c30c543317c")} aria-label={mode==='favorites'?localeText("main", "ui.WorkspaceLayout.622b9fa1c732d6ad"):mode==='ready'?localeText("main", "ui.WorkspaceLayout.39479039a9761983"):localeText("main", "ui.WorkspaceLayout.35dda167242a8168")}><Icon name={mode==='favorites'?'star':mode==='ready'?'check':'collection'}/><span>{mode==='favorites'?localeText("main", "ui.WorkspaceLayout.16cb64b29999f5f8"):mode==='ready'?localeText("main", "ui.WorkspaceLayout.49e199a4fd719d41"):localeText("main", "ui.WorkspaceLayout.103b204b23a77314")}</span></button></Hint>)}
        <Hint message={{id:"main.tab-recent"}}><button data-tour-id="main.workspace.recent" type="button" role="tab" className={recentOpen?'active':undefined} aria-selected={recentOpen} onClick={()=>{setRecentOpen(true);actions.closeContextMenu();}} title={localeText("main", "ui.WorkspaceLayout.1787dba90772999d")} aria-label={localeText("main", "ui.WorkspaceLayout.6fc2b8bbbf7846cc")}><Icon name="folder"/><span>{localeText("main", "ui.WorkspaceLayout.cb8e8fe08298f5fb")}</span></button></Hint>
        <span className="workspace-layout__title" aria-hidden="true" />
        {!recentOpen&&isFolderScanActive&&activeWorkspace==='explorer'?<span className="workspace-layout__scan-status" role="status">{localeText("main", "ui.WorkspaceLayout.9505736303ffb517")}</span>:null}
        {!recentOpen&&(activeWorkspace==='favorites'||activeWorkspace==='ready'||showingCollection||queryWorkspace&&search.needsRefresh)?<Hint message={{id:search.needsRefresh?'main.refresh-results-dirty':'main.refresh-results'}}><button type="button" className="workspace-layout__refresh-order" aria-label={localeText("main", "ui.WorkspaceLayout.28b76bb4f0dc1067")} title={search.needsRefresh?localeText("main", "ui.WorkspaceLayout.76240d09348110a8"):localeText("main", "ui.WorkspaceLayout.f28c52d77c4a869b")} disabled={search.isSearching} onClick={()=>actions.submitSearch(queryWorkspace?appliedOrder:undefined)}><Icon name="refresh"/>{search.needsRefresh?<span>{localeText("main", "ui.WorkspaceLayout.78431af6b46309de")}</span>:null}</button></Hint>:null}
        <div data-tour-id="main.display.overview" className="workspace-layout__view-controls" aria-label={messages().workspace.viewControls}>
          {comparisonCount===2&&!recentOpen?<Hint message={{id:"main.compare"}}><button data-tour-id="main.canvas.compare" type="button" className="secondary-action workspace-compare" aria-label={localeText("main", "ui.WorkspaceLayout.d917344b47a8edbf")} title={localeText("main", "ui.WorkspaceLayout.1204380a1d599be1")} disabled={comparisonBusy} onClick={()=>void compare()}><Icon name="image"/><span>{localeText("main", "ui.WorkspaceLayout.88859fed86b469de")}</span></button></Hint>:null}
          <CompactSelect tourId="main.display.fit" icon="imageFit" caption={localeText("main", "ui.WorkspaceLayout.1ab992931334ab5f")} className="compact-select--fit" value={uiPreferences.thumbnailFit} onChange={value=>saveUiPreferences({thumbnailFit:value as 'cover'|'contain'})} options={[{value:'cover',label:localeText("main", "ui.WorkspaceLayout.b836802d5f0d58de")},{value:'contain',label:localeText("main", "ui.WorkspaceLayout.b72872c7247b5c45")}]} message={{id:uiPreferences.thumbnailFit==='cover'?'main.fit-fill':'main.fit-contain'}}/>
          <CompactSelect tourId="main.display.view" helpId="view" caption={messages().toolbar.view} value={viewSize} onChange={value => actions.setViewSize(value as ViewSize)} options={[
            {value:'table',label:localeText("main", "ui.WorkspaceLayout.7a79a4464a19c6d7")},{value:'list',label:localeText("main", "ui.WorkspaceLayout.2df85c80e7067626")},
            ...(['small','medium','large','extraLarge'] as const).map(value => ({value,label:messages().toolbar.viewOptions[value]}))]} message={{id:viewSize==='table'?'main.view-table':viewSize==='list'?'main.view-list':'main.view-size'}} />
          <ExplorerSortSelect value={queryWorkspace?appliedOrder:sortMode} queryWorkspace={queryWorkspace} onChange={value=>applySort((queryWorkspace?sortForSearchOrder(value)??value:value) as SortMode)}/>
          {!queryWorkspace?<CompactSelect tourId="main.display.files" icon={showImagesOnly?'image':'file'} caption={localeText("main", "ui.WorkspaceLayout.b11d16e205fd92d1")} className="compact-select--files" value={showImagesOnly?"photos":"all"} onChange={value=>actions.setShowImagesOnly(value==="photos")} options={[{value:"photos",label:localeText("main", "ui.WorkspaceLayout.3bc8c6968b712bb4")},{value:"all",label:localeText("main", "ui.WorkspaceLayout.3eef40d085a0d930")}]} message={{id:"main.files"}}/>:null}
          {!queryWorkspace?<FilenameFilter value={filenameFilter} onChange={actions.setFilenameFilter} context={currentPath??''} loading={isFolderScanActive}/>:null}
        </div>
        {narrow?<Hint message={{id:"main.panel"}}><button type="button" className="workspace-details-toggle" aria-label={localeText("main", "ui.WorkspaceLayout.f403e36251d9d1b1")} aria-expanded={detailsOpen} onClick={onToggleDetails}><Icon name="info"/><span>{localeText("main", "ui.WorkspaceLayout.ca569c537dcd07b5")}</span></button></Hint>:null}
      </div>

       <div data-tour-id="main.workspace.content" data-help-id={activeWorkspace==='favorites'?'favorites-workspace':activeWorkspace==='ready'?'processed-workspace':activeWorkspace==='collections'?'collections-workspace':undefined} className={`workspace-layout__content${queryWorkspace?' workspace-layout__content--search':''}`} hidden={recentOpen}>
      {activeWorkspace==='collections'?<CollectionsWorkspace previewCollectionId={preview?search.collectionId:undefined}/>:null}
      {activeWorkspace === 'search' ? (
        <SearchControls
          currentPath={currentPath}
          onModeChange={setSavedSearchOpen}
          searchQuery={underlying?.query??searchQuery}
          recentSearches={recentSearches}
          searchPresets={searchPresets}
          search={underlying?.search??search}
          selectedSearchResultCount={selectedSearchResultCount}
          totalSearchResultCount={search.results.length}
          collections={collections}
          onQueryChange={actions.setSearchQuery}
          onSubmit={actions.submitSearch}
          onLoadMore={actions.loadMoreSearchResults}
          onClear={()=>actions.clearSearch(true)}
          onStatusFilterChange={actions.setSearchStatusFilter}
          onTagFilterTextChange={actions.setSearchTagFilterText}
          onTagModeChange={actions.setSearchTagMode}
          onAddTagFilter={actions.addSearchTagFilter}
          onRemoveTagFilter={actions.removeSearchTagFilter}
          onDateFromChange={actions.setSearchDateFrom}
          onDateToChange={actions.setSearchDateTo}
          onScopeChange={actions.setSearchScope}
          onQuickFilter={actions.applySearchQuickFilter}
          onSavePreset={actions.saveCurrentSearchPreset}
          onApplyPreset={actions.applySearchPreset}
          onDeletePreset={actions.deleteSearchPreset}
          onTogglePresetFavorite={actions.toggleSearchPresetFavorite}
          onRenamePreset={actions.renameSearchPreset}
          onExportResults={actions.exportSearchResults}
          onCopyPaths={(scope) => { void actions.copySearchResultPaths(scope); }}
          onRevealFolders={(scope) => { void actions.revealSearchResultFolders(scope); }}
          onCreateCollection={(name) => { void actions.createCollection(name); }}
          onAddSelectedToCollection={(collectionId) => { void actions.addSelectedImagesToCollection(collectionId); }}
          onReloadCollections={() => { void actions.loadCollections(); }}
          onRenameCollection={(id,name)=>{void actions.renameCollection(id,name);}}
          onDeleteCollection={(id)=>{void actions.deleteCollection(id);}}
          onRemoveSelectedFromCollection={(id)=>{void actions.removeSelectedFromCollection(id);}}
        />
      ) : null}

      {!quickActive&&activeWorkspace==='explorer' && !currentPath && !isScanningFolder ? <section className="details-section" aria-label={localeText("main", "ui.WorkspaceLayout.3dea12f9a932df1e")}>
        <h2>{localeText("main", "ui.WorkspaceLayout.232f89f8e1ecaf2f")}</h2>
        <p>{localeText("main", "ui.WorkspaceLayout.826e2dfb30523d6b")}</p>
        <div className="queue-control-row">
          <Hint message={{id:"main.choose-folder"}}><button type="button" className="primary-action" onClick={()=>{void actions.chooseFolder();}}>{localeText("main", "ui.WorkspaceLayout.61f0e1a037a7609f")}</button></Hint>
          <Hint message={{id:"local.setup"}}><button type="button" className="secondary-action" onClick={()=>{actions.setAiProviderMode('local');requestSettingsTab('recognition');actions.setSettingsPanelOpen(true);}}>{localeText("main", "ui.WorkspaceLayout.09a98f41190fbbff")}</button></Hint>
          <Hint message={{id:"main.provider"}}><button type="button" className="secondary-action" onClick={()=>{actions.setAiProviderMode('fal');requestSettingsTab('recognition');actions.setSettingsPanelOpen(true);}}>{localeText("main", "ui.WorkspaceLayout.49d691bb0e3fd450")}</button></Hint>
        </div>
      </section> : null}
      <div data-tour-id="main.canvas" className="workspace-grid-host" hidden={activeWorkspace==='search'&&savedSearchOpen}>
      {activeWorkspace!=='collections'||showingCollection?<ExplorerGrid
        scrollContext={quickActive?`quick:${search.queryId??''}`:activeWorkspace==='explorer'?`explorer:${currentPath??''}:filter:${filenameFilter}`:activeWorkspace}
        items={workspaceItems}
        selectedItemIds={selectedItemIds}
        focusedItemId={selectedItemId}
        viewSize={viewSize}
        sortMode={activeSort}
        onSort={applySort}
        isLoading={queryWorkspace ? search.isSearching&&!search.results.length : isScanningFolder}
        hasMore={queryWorkspace&&search.results.length<search.resultCount}
        loadingMore={search.isLoadingPage||search.isSearching}
        pageError={queryWorkspace?search.pageError:null}
        onReachEnd={queryWorkspace&&!help.open?actions.loadMoreSearchResults:undefined}
        errorMessage={queryWorkspace ? search.results.length?null:search.errorMessage : filesystemError}
        onSelect={actions.selectItem}
        onToggleSelection={actions.toggleItemSelection}
        onRangeSelect={actions.selectRangeToItem}
        onSelectAll={actions.selectAllVisible}
        onSelectAllItems={actions.selectAllInWorkspace}
        onOpenFolder={actions.openFolder}
        onOpenFile={openWorkspaceItem}
        onContextMenu={actions.openContextMenu}
        onToggleFavorite={(itemId) => { void actions.toggleFavoriteForItem(itemId); }}
        emptyTitle={queryWorkspace ? search.lastSearchedAtUnix?localeText("main", "ui.WorkspaceLayout.27b605b5a494dfb4"):localeText("main", "ui.WorkspaceLayout.c4a7ad026b441d21") : messages().workspace.explorerEmptyTitle}
        emptyMessage={activeWorkspace==='favorites'?localeText("main", "ui.WorkspaceLayout.6ca87f04758933ac"):activeWorkspace==='ready'?localeText("main", "ui.WorkspaceLayout.d69dc3016a921ee6"):activeWorkspace==='collections'?localeText("main", "ui.WorkspaceLayout.1bc9e1c54bedc763"):activeWorkspace === 'search' ? messages().workspace.searchResultsEmptyMessage : messages().workspace.explorerEmptyMessage}
      />:null}
      {viewSize!=='table'&&(activeWorkspace!=='collections'||showingCollection)?<span className="canvas-selection-help"><InfoHint label={localeText("main", "ui.WorkspaceLayout.fecab4bc4a4d8f41")} message={{id:'main.card-select'}} activationOnly/></span>:null}
      </div>
      {comparisonError?<p role="alert">{comparisonError}</p>:null}
      </div>
      {recentOpen?<section data-help-id="recent-workspace" className="recent-folders-panel" role="tabpanel" aria-label={localeText("main", "ui.WorkspaceLayout.6fc2b8bbbf7846cc")}><h2>{localeText("main", "ui.WorkspaceLayout.cb8e8fe08298f5fb")}</h2><p>{localeText("main", "ui.WorkspaceLayout.c508cc0560817eff")}</p>{recentPaths.length?<div className="recent-folders-panel__list">{recentPaths.map(path=><Hint key={path} message={{id:"main.open-recent",args:{папка:path}}}><button key={path} type="button" onClick={()=>{setRecentOpen(false);void actions.openBreadcrumb(path);}}><span className="recent-folders-panel__icon" aria-hidden="true"><Icon name="folder"/></span><span><strong>{path.replaceAll('\\','/').split('/').filter(Boolean).at(-1)??path}</strong><small>{path}</small></span><Icon name="right"/></button></Hint>)}</div>:<div className="empty-state"><strong>{localeText("main", "ui.WorkspaceLayout.97b2f14daaf1eb7c")}</strong><span>{localeText("main", "ui.WorkspaceLayout.52509b13153d6310")}</span></div>}</section>:null}


      <ExplorerContextMenu
        menu={contextMenu}
        targetItem={contextMenuItem}
        selectedCount={queryWorkspace?selectedSearchResultCount:selectedItems.length}
        selectedImageCount={queryWorkspace?selectedSearchResultCount:selectedImageItems.length}
        visibleItemIds={visibleItemIds}
        onOpenTarget={() => { if (contextMenuItem) { openWorkspaceItem(contextMenuItem.id); } actions.closeContextMenu(); }}
        onRevealTarget={() => { if (contextMenuItem) { actions.revealSelectedInFileManager(contextMenuItem.id); } actions.closeContextMenu(); }}
        onSelectAll={actions.selectAllInWorkspace}
        onClearSelection={actions.clearSelection}
        onQueueSelected={actions.queueSelectedForProcessing}
        onClose={actions.closeContextMenu}
        onEditSelected={() => { actions.closeContextMenu(); void actions.openSelectedPhotoEditor(); }}
        onCompare={()=>void compare()}
      />
      {comparison?<ImageComparison images={comparison} onClose={()=>setComparison(null)}/>:null}
    </section>
  );
}

const MemoMainPane = memo(WorkspaceMainPane);
const MemoDetailsPane = memo(WorkspaceDetailsPane);

function WorkspaceDetailsPane() {
  const help=useHelpPresentation();
  const actions = useAppActions();
  const selectedItem = useAppSelector(selectSelectedItem);
  const selectedItems = useAppSelector(selectSelectedItems, sameItemArray);
  const taskSummary = useAppSelector((state) => state.taskSummary);
  const taskQueue = useAppSelector((state) => state.taskQueue);
  const indexDatabase = useAppSelector((state) => state.indexDatabase);
  const activeDetailsTab = useAppSelector((state) => state.activeDetailsTab);
  const collections = useAppSelector((state) => state.collections);
  const processingSelectionDraft = useAppSelector((state) => state.processingSelectionDraft);
  const renamePreview = useAppSelector((state) => state.renamePreview);

  return (
    <DetailsPanel
      selectedItem={selectedItem}
      selectedItems={selectedItems}
      taskSummary={taskSummary}
      taskQueue={taskQueue}
      indexDatabase={indexDatabase}
      activeTab={activeDetailsTab}
      previewTab={help.view.detailsTab}
      collections={collections}
      processingSelectionDraft={processingSelectionDraft}
      renamePreview={renamePreview}
      onSaveImageMetadata={actions.updateImageMetadata}
      onToggleFavorite={(itemId) => { void actions.toggleFavoriteForItem(itemId); }}
      onCreateCollection={(name) => { void actions.createCollection(name); }}
      onAddSelectedToCollection={(collectionId) => { void actions.addSelectedImagesToCollection(collectionId); }}
      onReloadCollections={() => { void actions.loadCollections(); }}
      onStartTaskQueue={actions.startTaskQueue}
      onPauseTaskQueue={actions.pauseTaskQueue}
      onResumeTaskQueue={actions.resumeTaskQueue}
      onStopTaskQueue={actions.stopTaskQueue}
      onClearFinishedTasks={actions.clearFinishedTasks}
      onRetryFailedTasks={actions.retryFailedTasks}
      onRescanIndex={actions.rescanIndex}
      onConfirmRenamePreview={actions.confirmRenamePreviewQueue}
      onCancelRenamePreview={actions.cancelRenamePreview}
      onTabChange={actions.setDetailsTab}
    />
  );
}
