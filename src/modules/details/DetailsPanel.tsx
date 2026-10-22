import {captureDateLabel} from '../filesystem/ImageMetadata';
import {nativeDisplayText} from "../../shared/i18n/NativeMessages";
import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {plural} from '../../shared/i18n/Formatting';
import {text as localeText} from "../../shared/i18n/Runtime";
import {useCurrentLocale} from "../../shared/i18n/Runtime";
import {useHelpDisclosure,useHelpPresentation} from '../help/HelpPresentation';
import {Hint} from "../../shared/help/Hint";
import {sectionHelp} from '../../shared/help/SelectionHelp';
import {journalFileName,journalPresentation} from '../actionLog/JournalPresentation';
import {EditorModal} from '../photoEditor/EditorModal';
import {listen} from '@tauri-apps/api/event';
import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import type { DetailsTab, ProcessingSelectionDraft, RenamePreviewState } from '../../app/types';
import type { ExplorerItem, FileItem, ImageIndexState, ImageItem, ProcessingStatus } from '../explorer/types';
import type { TaskQueueState, TaskSummary } from '../taskQueue/types';
import { summarizeTaskQueue } from '../taskQueue/TaskQueueService';
import type { IndexDatabaseState, IndexMaintenanceImage, IndexMaintenanceResult, IndexMetadataUpdateInput } from '../index/types';
import { indexService } from '../index/IndexService';
import { fileOpenService } from '../filesystem/FileOpenService';
import {ZoomPreview} from './ZoomPreview';
import { CachedImage } from '../thumbnail/CachedImage';
import { actionLogService } from '../actionLog/ActionLogService';
import type { ActionJournalEntry } from '../actionLog/ActionLog.types';
import { RenamePreviewPanel } from '../rename/RenamePreviewPanel';
import type { CollectionsState } from '../collections/types';
import { messages, pluralizeRu } from '../../shared/i18n';
import { useMetadataDraft } from './MetadataDraft';
import { useAppActions, useAppDispatch } from '../../app/state/AppStateContext';
import { ProcessButton } from '../../shared/ui/ProcessButton';
import { Icon } from '../../shared/ui/Icon';
import {PhotoCollections} from './PhotoCollections';
import {DetailsPreviewSize} from './DetailsPreviewSize';
import {DescriptionEditor} from './DescriptionEditor';
import {invoke} from "../../shared/tauri/invoke";
import {openSmartImageViewer} from '../imageViewer';
import {selectViewerImages} from '../../app/state/appSelectors';
import { BatchTagsPanel } from './BatchTagsPanel';
import { IndexBackupPanel } from './IndexBackupPanel';
import { LibraryRootsPanel } from './LibraryRootsPanel';
import{ProjectStoragePanel}from'./ProjectStoragePanel';
import { DuplicatesPanel } from './DuplicatesPanel';
import { ThumbnailCachePanel } from './ThumbnailCachePanel';
import { useAppSelector } from '../../app/state/AppStateContext';
import {QueueSelectionCost,QueueCostStatistics} from './QueueCostSummary';
import {QueueSettingsPanel} from './QueueSettingsPanel';
import {QueueCapsules} from './QueueCapsules';
import {editorService} from '../photoEditor/EditorService';
import {QueueHistory} from './QueueHistory';
import {QueueRemoveButton} from './QueueRemoveButton';
import {QueueListControls} from './QueueListControls';
import {InfoHint,HintTitle} from '../../shared/ui/InfoHint';
import {sectionHints} from '../../shared/ui/SectionHints';

interface DetailsPanelProps {
  selectedItem: ExplorerItem | null;
  selectedItems: ExplorerItem[];
  taskSummary: TaskSummary;
  taskQueue: TaskQueueState;
  indexDatabase: IndexDatabaseState;
  processingSelectionDraft: ProcessingSelectionDraft;
  renamePreview: RenamePreviewState;
  activeTab: DetailsTab;
  previewTab?:DetailsTab;
  collections: CollectionsState;
  onSaveImageMetadata(input: IndexMetadataUpdateInput): Promise<void>;
  onToggleFavorite(itemId: string): void;
  onCreateCollection(name: string): void;
  onAddSelectedToCollection(collectionId: string): void;
  onReloadCollections(): void;
  onStartTaskQueue(): void;
  onPauseTaskQueue(): void;
  onResumeTaskQueue(): void;
  onStopTaskQueue(): void;
  onClearFinishedTasks(): void;
  onRetryFailedTasks(): void;
  onRescanIndex(): Promise<void>;
  onConfirmRenamePreview(): void;
  onCancelRenamePreview(): void;
  onTabChange(tab: DetailsTab): void;
}

const statusLabel: Record<ProcessingStatus, string> = {
  get processed() { return messages().explorer.statusLabels.processed; },
  get queued() { return messages().explorer.statusLabels.queued; },
  get notProcessed() { return messages().explorer.statusLabels.notProcessed; },
  get error() { return messages().explorer.statusLabels.error; },
  get relocated() { return messages().explorer.statusLabels.relocated; },
  get missing() { return messages().explorer.statusLabels.missing; },
};

const indexStateLabel: Record<ImageIndexState, string> = {
  get new() { return messages().details.indexStateNew; },
  get known() { return messages().details.indexStateKnown; },
  get pathUpdated() { return messages().details.indexStatePathUpdated; },
};

function isImage(item: ExplorerItem | null): item is ImageItem {
  return item?.kind === 'image';
}

function isFile(item: ExplorerItem | null): item is FileItem {
  return item?.kind === 'file';
}

export function DetailsPanel(props: DetailsPanelProps) {
  useCurrentLocale();
  const reconciliationActions=useAppActions();
  useEffect(()=>{let disposed=false;let off:(()=>void)|undefined;void listen('smart-photo-indexer:editor-reconciliation',()=>{if(!disposed)reconciliationActions.setDetailsTab('tasks');}).then(f=>disposed?f():off=f);return()=>{disposed=true;off?.();};},[reconciliationActions]);
  const {
    selectedItem,
    selectedItems,
    taskQueue,
    indexDatabase,
    activeTab,previewTab,
    processingSelectionDraft,
    renamePreview,
    onSaveImageMetadata,
    onToggleFavorite,
    onStartTaskQueue,
    onPauseTaskQueue,
    onResumeTaskQueue,
    onStopTaskQueue,
    onClearFinishedTasks,
    onRetryFailedTasks,
    onRescanIndex,
    onConfirmRenamePreview,
    onCancelRenamePreview,
    onTabChange,
  } = props;

  const renderTasks = () => (
    <TasksTabContent
      taskQueue={taskQueue}
      processingSelectionDraft={processingSelectionDraft}
      renamePreview={renamePreview}
      onConfirmRenamePreview={onConfirmRenamePreview}
      onCancelRenamePreview={onCancelRenamePreview}
      onStart={onStartTaskQueue}
      onPause={onPauseTaskQueue}
      onResume={onResumeTaskQueue}
      onStop={onStopTaskQueue}
      onClearFinished={onClearFinishedTasks}
      onRetryFailed={onRetryFailedTasks}
    />
  );

  const renderIndex = () => <IndexTabContent indexDatabase={indexDatabase} onRescanIndex={onRescanIndex} />;

  const renderContent = (activeTab:DetailsTab) => {
    if (!selectedItem) {
      if (activeTab === 'tasks') return renderTasks();
      if (activeTab === 'index') return renderIndex();
      if (activeTab === 'journal') return <JournalTabContent selectedItem={null} />;
      return <div className="empty-details">{messages().details.noSelectionHint}</div>;
    }

    if (isFile(selectedItem)) {
      if (activeTab === 'tasks') return renderTasks();
      if (activeTab === 'index') return renderIndex();
      if (activeTab === 'journal') return <JournalTabContent selectedItem={null} />;
      return (
        <>
          <div data-tour-id="main.details.preview" className="details-preview details-preview--file" aria-hidden="true">{selectedItem.fileType}</div>
          <SelectionSummary selectedItems={selectedItems} />
          <FileDetailsTabContent selectedItem={selectedItem} />
        </>
      );
    }

    if (!isImage(selectedItem)) {
      if (activeTab === 'tasks') return renderTasks();
      if (activeTab === 'index') return renderIndex();
      if (activeTab === 'journal') return <JournalTabContent selectedItem={null} />;
      return (
        <>
          <SelectionSummary selectedItems={selectedItems} />
          <div className="empty-details">{messages().details.folderSelected(selectedItem.name)}</div>
        </>
      );
    }

    const showImageHeader = activeTab === 'details';

    return (
      <>

        {showImageHeader?<ImagePreviewHeader selectedItem={selectedItem} onToggleFavorite={onToggleFavorite}/>:null}
        {['details','aiDescription','tags'].includes(activeTab)?<SelectedPhotoActions item={selectedItem} showPreview={activeTab!=='details'}/>:null}
        {activeTab==='details'?<SelectionSummary selectedItems={selectedItems}/>:null}
        {activeTab === 'details' ? <><DetailsTabContent selectedItem={selectedItem} /><PhotoCollections items={selectedItems.filter((item):item is ImageItem=>item.kind==='image')}/></> : null}
        {activeTab === 'aiDescription' ? <AiDescriptionTabContent selectedItem={selectedItem} onSaveImageMetadata={onSaveImageMetadata} /> : null}
        {activeTab === 'tags' ? <><TagsTabContent selectedItem={selectedItem} onSaveImageMetadata={onSaveImageMetadata} /><BatchTagsPanel items={selectedItems} /></> : null}
        {activeTab === 'tasks' ? renderTasks() : null}
        {activeTab === 'index' ? renderIndex() : null}
        {activeTab === 'journal' ? <JournalTabContent selectedItem={selectedItem} /> : null}
      </>
    );
  };

  return (
    <aside data-tour-id="main.inspector.overview" className="details-panel">
      <PanelTabs activeTab={previewTab??activeTab} onTabChange={onTabChange} />
      <div data-tour-id={activeTab==='details'?'main.details.overview':undefined} hidden={Boolean(previewTab&&previewTab!==activeTab)} data-help-id={!selectedItem?(activeTab==='aiDescription'?'description-panel':activeTab==='tags'?'tags-panel':undefined):undefined} className={`details-panel__content details-panel__content--${activeTab}`} role="tabpanel">
        {renderContent(activeTab)}
      </div>
      {previewTab&&previewTab!==activeTab?<div data-tour-id={previewTab==='details'?'main.details.overview':undefined} key={'help-'+previewTab} data-tour-preview className={'details-panel__content details-panel__content--'+previewTab} role="tabpanel">{renderContent(previewTab)}</div>:null}
    </aside>
  );
}

function ImagePreviewHeader({ selectedItem, onToggleFavorite }: { selectedItem: ImageItem; onToggleFavorite(itemId: string): void }) {
  useCurrentLocale();
  const [opening,setOpening]=useState(false),[error,setError]=useState('');
  return (
    <div data-tour-id="main.details.preview" className="details-image-header">
      <Hint message={{id:"main.edit-photo"}}><button type="button" className="details-edit-photo" aria-label={localeText("details", "ui.DetailsPanel.a0c7e58e55513e7a")} title={localeText("details", "ui.DetailsPanel.3b083903f541c0aa")} disabled={opening} onClick={()=>{setOpening(true);setError('');void editorService.open(selectedItem.path).catch(e=>setError(String(e))).finally(()=>setOpening(false));}}><Icon name="edit"/>{localeText("details", "ui.DetailsPanel.276d5f6ecee786b4")}</button></Hint>
      {error?<span role="alert" className="details-editor-error">{error}</span>:null}
      <DetailsPreviewSize><ZoomPreview key={selectedItem.id} item={selectedItem}/></DetailsPreviewSize>
      <Hint message={{id:selectedItem.isFavorite?'common.favorite-remove':'common.favorite-add'}}><button
        type="button"
        className={selectedItem.isFavorite ? 'details-favorite-overlay active' : 'details-favorite-overlay'}
        title={selectedItem.isFavorite ? messages().details.inFavorites : messages().details.addToFavorites}
        aria-label={selectedItem.isFavorite ? messages().details.inFavorites : messages().details.addToFavorites}
        onClick={() => onToggleFavorite(selectedItem.id)}
      >
        <Icon name="star" filled={Boolean(selectedItem.isFavorite)}/>
      </button></Hint>
    </div>
  );
}

function SelectedPhotoActions({item,showPreview}:{item:ImageItem;showPreview:boolean}){
  useCurrentLocale();
  const actions=useAppActions();const [message,setMessage]=useState('');
   const viewerImages=useAppSelector(selectViewerImages,(left,right)=>left.length===right.length&&left.every((item,index)=>item===right[index]));
  const query=useAppSelector(state=>state.activeWorkspace!=='explorer'&&Boolean(state.search.queryId));
  const copy=async()=>{try{await navigator.clipboard.writeText(item.path);setMessage(localeText("details", "ui.DetailsPanel.d647ea5bdcf11113"));}catch{setMessage(localeText("details", "ui.DetailsPanel.7a9fa9a208151b6a"));}};
  return <div className={`selected-photo-actions${showPreview?'':' selected-photo-actions--without-preview'}`}>
    {showPreview?<CachedImage imagePath={item.path} originalPreviewUrl={item.previewUrl} className="selected-photo-actions__preview" width={120} height={100} variant="details"/>:null}
    <div className="selected-photo-actions__name"><strong title={item.name}>{item.name}</strong><Hint message={{id:item.isFavorite?'common.favorite-remove':'common.favorite-add'}}><button type="button" className={item.isFavorite?'photo-favorite active':'photo-favorite'} aria-label={item.isFavorite?localeText("details", "ui.DetailsPanel.6d572eb3bf9e75d3"):localeText("details", "ui.DetailsPanel.d6844254cf193b43")} onClick={()=>void actions.toggleFavoriteForItem(item.id)}><Icon name="star" filled={Boolean(item.isFavorite)}/></button></Hint></div>
    <div className="selected-photo-actions__buttons"><ProcessButton compact/>
      <Hint message={{id:"common.reveal-file"}}><button type="button" aria-label={localeText("details", "ui.DetailsPanel.767b1b8e2ce784cb")} title={localeText("details", "ui.DetailsPanel.da1f01ba8013fa25")} onClick={()=>actions.revealSelectedInFileManager(item.id)}><Icon name="folder"/></button></Hint>
      <Hint message={{id:"common.copy-path"}}><button type="button" aria-label={localeText("details", "ui.DetailsPanel.1e2dc4eacfb385e0")} title={localeText("details", "ui.DetailsPanel.b196354dca27e85d")} onClick={()=>void copy()}><Icon name="copy"/></button></Hint>
      <Hint message={{id:"details.photo-open"}}><button type="button" aria-label={localeText("details", "ui.DetailsPanel.f7daf1831eccced5")} title={localeText("details", "ui.DetailsPanel.0e1158b95c3c7554")} onClick={()=>{if(query)void actions.openSearchViewer(item.id);else void openSmartImageViewer(item,viewerImages);}}><Icon name="image"/></button></Hint>
    </div>{message?<span className="photo-action-message" role="status">{message}</span>:null}
  </div>;
}

function Disclosure({title,children,id,initialOpen=false,hint,actions}:{title:string;children:ReactNode;id?:string;initialOpen?:boolean;hint?:string;actions?:ReactNode}){
  useCurrentLocale();
  const [open,setOpen]=useState(initialOpen);
  const revealed=useHelpDisclosure(id),help=useHelpPresentation();
  const explanation=sectionHints[id??'']??hint;
  return <details className="details-disclosure" open={open||revealed} onToggle={event=>{if(!help.open)setOpen(event.currentTarget.open);}}><Hint message={{id:"common.expand",args:{раздел:title}}}><summary data-tour-id={({'queue-items':'main.queue.list','queue-details':'main.queue.summary','queue-settings':'main.queue.parameters','queue-history':'main.queue.history','library-roots':'main.library.roots','index-maintenance':'main.library.maintenance','index-duplicates':'main.library.duplicates','index-cache':'main.library.cache','index-technical':'main.library.technical','project-storage':'main.library.projects'} as Record<string,string>)[id??'']} data-help-id={id}><span className="section-title"><span className="section-title__text">{title}</span>{explanation?<InfoHint label={localeText("details", "ui.DetailsPanel.e890d379764dbc67", {"value1": (title)})} text={explanation} message={sectionHelp(id)}/>:null}</span>{actions?<span className="details-disclosure-actions" onClick={e=>e.stopPropagation()} onKeyDown={e=>e.stopPropagation()}>{actions}</span>:null}<Icon name="down"/></summary></Hint><div className="details-disclosure__body">{children}</div></details>;
}

function SelectionSummary({ selectedItems }: { selectedItems: ExplorerItem[] }) {
  useCurrentLocale();
  const snapshot=useAppSelector(state=>state.activeWorkspace!=='explorer'?state.search.selectionSnapshot:undefined);
  if(snapshot)return <section className="details-section"><strong>{localeText("details", "ui.DetailsPanel.a606a58d6fd46ca1")}{' '}{snapshot.totalCount-snapshot.excludedPaths.length} {' '}{localeText("details", "ui.DetailsPanel.8af72daba60d421c")}</strong></section>;
  if (selectedItems.length <= 1) return null;

  const imageCount = selectedItems.filter((item) => item.kind === 'image').length;
  const folderCount = selectedItems.filter((item) => item.kind === 'folder').length;
  const fileCount = selectedItems.filter((item) => item.kind === 'file').length;

  return (
    <section className="details-section selection-summary-section">
      <h3>{messages().details.currentSelection}</h3>
      <div className="selection-summary-pills">
        <span className="tag-chip">{plural('items',selectedItems.length)}</span>
        {imageCount > 0 ? <span className="tag-chip">{plural('images',imageCount)}</span> : null}
        {folderCount > 0 ? <span className="tag-chip">{plural('folders',folderCount)}</span> : null}
        {fileCount > 0 ? <span className="tag-chip">{plural('files',fileCount)}</span> : null}
      </div>
    </section>
  );
}

function DetailsTabContent({ selectedItem }: { selectedItem: ImageItem }) {
  useCurrentLocale();
  return (
    <section className="details-section metadata-section">
      <dl data-tour-id="main.details.file-info">
        <div><dt>{messages().details.filename}:</dt><dd>{selectedItem.name}</dd></div>
        <div><dt>{messages().details.path}:</dt><dd>{selectedItem.path}</dd></div>
        <div><dt>{messages().details.status}:</dt><dd><i className={`dot ${selectedItem.status}`} /> {statusLabel[selectedItem.status]}</dd></div>
        <div><dt>{messages().details.favorite}:</dt><dd>{selectedItem.isFavorite ? messages().common.yes : messages().common.no}</dd></div>
        <div><dt>{messages().details.index}:</dt><dd>{selectedItem.indexState ? indexStateLabel[selectedItem.indexState] : messages().details.indexSyncPending}</dd></div>
        {selectedItem.previousPath ? <div><dt>{messages().details.previousPath}:</dt><dd>{selectedItem.previousPath}</dd></div> : null}
        <div><dt>{localeText("details", "ui.DetailsPanel.1f4f49a34335d812")}</dt><dd>{selectedItem.fileMetadata!==undefined?captureDateLabel(selectedItem.fileMetadata):(selectedItem.dateTaken||captureDateLabel(undefined))}</dd></div>
        <div><dt>{messages().details.dateModified}:</dt><dd>{selectedItem.modifiedAtUnix ? new Date(selectedItem.modifiedAtUnix*1000).toLocaleString(displayLocaleTag()) : localeText("details", "ui.DetailsPanel.1040e8890d6f9bf1")}</dd></div>
        <div><dt>{localeText("details", "ui.DetailsPanel.4454b596005f3ec7")}</dt><dd>{selectedItem.firstSeenAtUnix ? new Date(selectedItem.firstSeenAtUnix*1000).toLocaleString(displayLocaleTag()) : localeText("details", "ui.DetailsPanel.f199cdd4aab9b7c8")}</dd></div>
        {selectedItem.fileMetadata?.cameraMake || selectedItem.fileMetadata?.cameraModel ? <div><dt>{localeText("details", "ui.DetailsPanel.5bb56675255949d5")}</dt><dd>{[selectedItem.fileMetadata.cameraMake,selectedItem.fileMetadata.cameraModel].filter(Boolean).join(' ')}</dd></div> : null}
        {selectedItem.fileMetadata?.colorProfile?<div><dt>{localeText("details", "ui.DetailsPanel.0aee40164f06a2d8")}</dt><dd>{({absent:localeText("details", "ui.DetailsPanel.739ee33687392ca1"),rgbToSrgb:localeText("details", "ui.DetailsPanel.cb209fd1a42a2b61"),unsupportedOrInvalid:localeText("details", "ui.DetailsPanel.e00db67ba8d8dfac"),outsideLimit:localeText("details", "ui.DetailsPanel.fc74312545b40b31"),unreadable:localeText("details", "ui.DetailsPanel.f11477926ae24219"),unknown:localeText("details", "ui.DetailsPanel.a9313813049bf52b")} as Record<string,string>)[selectedItem.fileMetadata.colorProfile]??localeText("details", "ui.DetailsPanel.dff8f5a24bf7ce8b")}</dd></div>:null}
        {selectedItem.fileMetadata?.framePolicy==='firstFrame'?<div><dt>{localeText("details", "ui.DetailsPanel.74885fecf280873d")}</dt><dd>{localeText("details", "ui.DetailsPanel.9d06ada2744c370f")}</dd></div>:null}
        {selectedItem.fileMetadata?.framePolicy==='firstPage'?<div><dt>{localeText("details", "ui.DetailsPanel.74885fecf280873d")}</dt><dd>{localeText("details", "ui.DetailsPanel.a9288cbe5d1495a1")}</dd></div>:null}
        {selectedItem.fileMetadata?.decodeSupport==='unsupported'?<div><dt>{localeText("details", "ui.DetailsPanel.233305f35d8b73d9")}</dt><dd>{localeText("details", "ui.DetailsPanel.be8ba98964627c15")}</dd></div>:null}
        <div><dt>{messages().details.size}:</dt><dd>{selectedItem.sizeLabel} ({selectedItem.dimensions})</dd></div>
      </dl>
    </section>
  );
}

function FileDetailsTabContent({ selectedItem }: { selectedItem: FileItem }) {
  useCurrentLocale();
  return (
    <section className="details-section metadata-section">
      <dl>
        <div><dt>{messages().details.filename}:</dt><dd>{selectedItem.name}</dd></div>
        <div><dt>{messages().details.path}:</dt><dd>{selectedItem.path}</dd></div>
        <div><dt>{messages().details.fileType}:</dt><dd>{selectedItem.fileType}</dd></div>
        <div><dt>{messages().details.dateModified}:</dt><dd>{selectedItem.modifiedAt}</dd></div>
        <div><dt>{messages().details.size}:</dt><dd>{selectedItem.sizeLabel}</dd></div>
      </dl>
    </section>
  );
}

function AiDescriptionTabContent({ selectedItem, onSaveImageMetadata }: { selectedItem: ImageItem; onSaveImageMetadata(input: IndexMetadataUpdateInput): Promise<void> }) {
  useCurrentLocale();
  const saveOperation=useRef(0),mounted=useRef(true);
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;saveOperation.current++;};},[]);
  const draft = useMetadataDraft(selectedItem, 'description');
  const description = draft.value;
  const setDescription = draft.update;
  const [saveError, setSaveError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const hasChanges = description.trim() !== (selectedItem.description ?? '').trim()||JSON.stringify(draft.format??null)!==JSON.stringify(selectedItem.descriptionFormat??null);

  useEffect(() => {
    saveOperation.current++;
    setIsSaving(false);
    setSaveError(null);
  }, [selectedItem.id]);

  const save = async () => {
    const request=++saveOperation.current;
    setIsSaving(true);
    try {
      await onSaveImageMetadata({
        fields: ['description'],
        expectedDescriptionVersion: draft.version,
        imageId: selectedItem.indexedImageId ?? null,
        path: selectedItem.path,
        description,
        descriptionFormat:draft.format??null,
        tags: selectedItem.tags,
      });
      draft.saved();
      if(mounted.current&&request===saveOperation.current)setSaveError(null);
    } catch (error) {
      if(mounted.current&&request===saveOperation.current)setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      if(mounted.current&&request===saveOperation.current)setIsSaving(false);
    }
  };

  return (
    <section data-tour-id="main.description.overview" data-help-id="description-panel" className="details-section editable-metadata-section">
      <div className="section-header">
        <h3 className="section-title"><span>{messages().details.aiDescription}</span><InfoHint label={localeText("details", "ui.DetailsPanel.04d8d73b4ccf34eb")}  message={{id:"details.description"}}/></h3>
        <span className="metadata-edit-badge">{messages().details.manualEdit}</span>
      </div>
      <DescriptionEditor key={selectedItem.indexedImageId??selectedItem.path} text={description} format={draft.format} disabled={!selectedItem.indexedImageId||draft.sourceChanged} onChange={setDescription}/>
      <div className="metadata-edit-actions">
        <Hint message={{id:"common.reset-draft"}}><button type="button" className="secondary-action" disabled={(!hasChanges && !draft.sourceChanged) || isSaving} onClick={draft.reset}>{messages().details.reset}</button></Hint>
        <Hint message={{id:"details.save-description"}}><button data-tour-id="main.description.save" type="button" className="primary-action" disabled={!hasChanges || isSaving || draft.sourceChanged || !selectedItem.indexedImageId} onClick={save}>{isSaving ? messages().common.saving : messages().details.saveDescription}</button></Hint>
      </div>
      {draft.sourceChanged ? <p role="alert">{localeText("details", "ui.DetailsPanel.dd23350aefa68c40")}</p> : null}
      {draft.conflict ? <p role="status">{localeText("details", "ui.DetailsPanel.48d4083ffd883812")}</p> : null}
      {saveError ? <p role="alert">{saveError}</p> : null}
    </section>
  );
}

function TagsTabContent({ selectedItem, onSaveImageMetadata }: { selectedItem: ImageItem; onSaveImageMetadata(input: IndexMetadataUpdateInput): Promise<void> }) {
  useCurrentLocale();
  const saveOperation=useRef(0),mounted=useRef(true);
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;saveOperation.current++;};},[]);
  const draft = useMetadataDraft(selectedItem, 'tags');
  const tagText = draft.value;
  const setTagText = draft.update;
  const [saveError, setSaveError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const parsedTags = useMemo(() => parseTags(tagText), [tagText]);
  const originalTags = useMemo(() => normalizeComparableTags(selectedItem.tags), [selectedItem.tags]);
  const hasChanges = parsedTags.join('|') !== originalTags.join('|');

  useEffect(() => {
    saveOperation.current++;
    setIsSaving(false);
    setSaveError(null);
  }, [selectedItem.id]);

  const save = async () => {
    const request=++saveOperation.current;
    setIsSaving(true);
    try {
      await onSaveImageMetadata({
        fields: ['tags'],
        expectedTagsVersion: draft.version,
        imageId: selectedItem.indexedImageId ?? null,
        path: selectedItem.path,
        description: selectedItem.description,
        tags: parsedTags,
      });
      draft.saved();
      if(mounted.current&&request===saveOperation.current)setSaveError(null);
    } catch (error) {
      if(mounted.current&&request===saveOperation.current)setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      if(mounted.current&&request===saveOperation.current)setIsSaving(false);
    }
  };

  return (
    <section data-tour-id="main.tags.overview" data-help-id="tags-panel" className="details-section editable-metadata-section">
      <div className="section-header">
        <h3>{messages().details.aiTags}</h3>
        <span className="metadata-edit-badge">{messages().details.manualEdit}</span>
      </div>
      <div className="tag-list tag-list--preview">
        {parsedTags.length === 0 ? <span className="tag-chip tag-chip--muted">{messages().details.noTagsYet}</span> : null}
        {parsedTags.map((tag) => <span key={tag} className="tag-chip">{tag}<Hint message={{id:"details.tag-remove",args:{тег:tag}}}><button type="button" aria-label={localeText("details", "ui.DetailsPanel.c2ed22973a08421b", {"value1": (tag)})} disabled={draft.sourceChanged || !selectedItem.indexedImageId} onClick={()=>setTagText(parsedTags.filter(value=>value!==tag).join(', '))}><Icon name="close"/></button></Hint></span>)}
      </div>
      <textarea
        className="metadata-textarea metadata-textarea--tags"
        disabled={!selectedItem.indexedImageId}
        readOnly={draft.sourceChanged}
        value={tagText}
        placeholder={localeText("details", "ui.DetailsPanel.ed71fc418214ebdc")}
        rows={5}
        onChange={(event) => setTagText(event.target.value)}
      />
      <div className="metadata-edit-actions">
        <Hint message={{id:"common.reset-draft"}}><button type="button" className="secondary-action" disabled={(!hasChanges && !draft.sourceChanged) || isSaving} onClick={draft.reset}>{messages().details.reset}</button></Hint>
        <Hint message={{id:"details.save-tags"}}><button data-tour-id="main.tags.single" type="button" className="primary-action" disabled={!hasChanges || isSaving || draft.sourceChanged || !selectedItem.indexedImageId} onClick={save}>{isSaving ? messages().common.saving : messages().details.saveTags}</button></Hint>
      </div>
      <p className="metadata-edit-note">{messages().details.saveTagsNote}</p>
      {draft.sourceChanged ? <p role="alert">{localeText("details", "ui.DetailsPanel.7a8e382efe01595a")}</p> : null}
      {draft.conflict ? <p role="status">{localeText("details", "ui.DetailsPanel.65ee1d6421aa66f6")}</p> : null}
      {saveError ? <p role="alert">{saveError}</p> : null}
    </section>
  );
}

function TasksTabContent({
  taskQueue,
  processingSelectionDraft,
  renamePreview,
  onConfirmRenamePreview,
  onCancelRenamePreview,
  onStart,
  onPause,
  onResume,
  onStop,
  onClearFinished,
  onRetryFailed,
}: {
  taskQueue: TaskQueueState;
  processingSelectionDraft: ProcessingSelectionDraft;
  renamePreview: RenamePreviewState;
  onConfirmRenamePreview(): void;
  onCancelRenamePreview(): void;
  onStart(): void;
  onPause(): void;
  onResume(): void;
  onStop(): void;
  onClearFinished(): void;
  onRetryFailed(): void;
}) {
  useCurrentLocale();
  const manual=useAppSelector(state=>state.settings.processingLimits.manualQueue);
  const preparationMode=taskQueue.pendingPreparation?.config.queueMode??taskQueue.preparationResult?.queueMode;
  const preparationInMode=!preparationMode||preparationMode===(manual?'manual':'direct');
  const otherActive=Math.max(taskQueue.nativeAggregate?.otherActive??0,!preparationInMode&&['preparing','starting'].includes(taskQueue.preparationState??'')?1:0);
  const counters = taskQueue.nativeAggregate?.counters??summarizeTaskQueue(taskQueue);
  const runningTask = taskQueue.tasks.find((task) => task.status === 'running') ?? null;
  const [clock,setClock]=useState(()=>Math.floor(Date.now()/1000));
  useEffect(()=>{if(!taskQueue.tasks.some(task=>task.status==='running'))return;const timer=window.setInterval(()=>setClock(Math.floor(Date.now()/1000)),1000);return()=>window.clearInterval(timer);},[taskQueue.tasks.some(task=>task.status==='running')]);
  const selectedCount=useAppSelector(state=>{if(state.activeWorkspace!=='explorer'&&state.search.selectionSnapshot)return state.search.selectionSnapshot.totalCount-state.search.selectionSnapshot.excludedPaths.length;const ids=new Set(state.selectedItemIds);return (state.activeWorkspace==='explorer'?state.explorerItems:state.search.results).filter(item=>item.kind==='image'&&ids.has(item.id)).length;});
  const native=taskQueue.nativeStatus??'';
  const preparing=taskQueue.preparationState==='preparing'&&preparationInMode;
  const aggregate=taskQueue.nativeAggregate;
  const canStart = aggregate?aggregate.ready>0&&!taskQueue.commandPending:Boolean(taskQueue.nativeJobId)&&native==='prepared'&&counters.pending>0&&!taskQueue.commandPending;
  const canPause = aggregate?Boolean(aggregate.active||aggregate.preflight||taskQueue.commandPending):native==='running'||Boolean(taskQueue.commandPending);
  const canResume = !aggregate&&['paused','interrupted','waitingService'].includes(native)&&!taskQueue.commandPending;
  const canStop = preparing||Boolean(taskQueue.commandPending)||(aggregate?aggregate.stoppable>0:Boolean(taskQueue.nativeJobId)&&['prepared','queued','running','paused','interrupted','waitingService'].includes(native));
  const canRetryFailed = counters.failed > 0 && counters.running === 0 && ['paused','completed'].includes(native)&&!taskQueue.commandPending;
  const stateText=preparing?localeText("details", "ui.DetailsPanel.8109c5143b5d25b6"):taskQueue.preparationState==='starting'&&preparationInMode?localeText("details", "ui.DetailsPanel.ae40f9f47cad65e3"):({preparationFailed:localeText("details", "ui.DetailsPanel.99eabd22736e653b"),prepared:localeText("details", "ui.DetailsPanel.fb51b772f3becae8"),queued:localeText("details", "ui.DetailsPanel.103da87c4cebb015"),running:localeText("details", "ui.DetailsPanel.582af4013dab887c"),paused:localeText("details", "ui.DetailsPanel.d067a9fc3bd57888"),interrupted:localeText("details", "ui.DetailsPanel.d69a2cda3193f895"),waitingService:localeText("details", "ui.DetailsPanel.82cd11a4e859bffc"),completed:localeText("details", "ui.DetailsPanel.f9db4973384dd506"),stopped:localeText("details", "ui.DetailsPanel.bff5b9ced41aa143")} as Record<string,string>)[native]??localeText("details", "ui.DetailsPanel.efcd936f18a8e2dd");

  return (
    <section className="details-section task-section">
      <div data-tour-id="main.queue.overview" data-help-id="queue" className="task-section__heading">
        <h3 className="section-title"><span>{localeText("details", "ui.DetailsPanel.27d1be7a42e58fe1")}</span><InfoHint label={localeText("details", "ui.DetailsPanel.7da97b559a3aced2")}  message={{id:"queue.section"}}/></h3>
        <span role="status" aria-label={stateText} className={`queue-state-badge queue-state-badge--${taskQueue.runState}`} title={taskQueue.lastMessage?`${stateText}\n${taskQueue.lastMessage}`:stateText}>{stateText}</span>
        <span className="task-selection-count" title={localeText("details", "ui.DetailsPanel.ddecc4accfbfef60", {"value1": (selectedCount)})} aria-label={localeText("details", "ui.DetailsPanel.e635a51ae34a2e3a", {"value1": (selectedCount)})}><span className="task-selection-count__caption">{localeText("details", "ui.DetailsPanel.8be015826309f419")}{' '}</span>{selectedCount}</span>
        <ProcessButton compact/>
        <QueueRemoveButton scope="allWorking"/>
      </div>

      <div data-tour-id="main.queue.controls" className="queue-control-row queue-primary-controls">
        <Hint message={{id:canPause?'queue.pause':canResume?'queue.resume':'queue.start'}}><button type="button" className={canPause?'secondary-action':'primary-action'} disabled={!canPause&&!canStart&&!canResume} onClick={canPause?onPause:canResume?onResume:onStart}><Icon name={canPause?'pause':'play'}/>{canPause?messages().details.pause:canResume?messages().details.resume:localeText("details", "ui.DetailsPanel.03fd53a55a2ed503")}</button></Hint>
        <Hint message={{id:"queue.stop"}}><button type="button" className="secondary-action danger-action" disabled={!canStop} title={preparing?localeText("details", "ui.DetailsPanel.1048d2c4043cc82e"):localeText("details", "ui.DetailsPanel.9bf21a6195b92319")} onClick={onStop}><Icon name="stop"/>{messages().details.stop}</button></Hint>
        <Hint message={{id:"queue.retry-failed"}}><button type="button" className="secondary-action queue-retry-button" aria-label={messages().details.retryFailed} title={messages().details.retryFailed} disabled={!canRetryFailed} onClick={onRetryFailed}><Icon name="refresh"/></button></Hint>
      </div>
      <QueueSelectionCost/>
      {otherActive>0?<p role="status" className="queue-other-mode">{localeText('details','queue.otherActive',{count:otherActive})}</p> :null}
      {taskQueue.lastErrorMessage?<p role="alert" className="queue-error">{taskQueue.lastErrorMessage}</p>:null}
      {native==='queued'?<p role="status">{localeText("details", "ui.DetailsPanel.3deb3d615c435912")}</p>:null}


      <RenamePreviewPanel renamePreview={renamePreview} taskQueue={taskQueue} onConfirm={onConfirmRenamePreview} onCancel={onCancelRenamePreview} />

      <div data-tour-id="main.queue.progress" data-help-id="queue-progress" className="queue-progress-panel"><div className="section-header task-progress-header">
        <HintTitle title={messages().details.overallProgress}  message={{id:"queue.progress"}}/>
        <strong>{counters.progressPercent}%</strong>
      </div>
      <progress aria-label={localeText("details", "ui.DetailsPanel.464222b391855a93")} value={counters.progressPercent} max="100" /></div>


      {processingSelectionDraft.skippedCount>0?<p>{localeText("details", "ui.DetailsPanel.8823012923ff2787")}{' '}{processingSelectionDraft.skippedCount}</p>:null}


      <Disclosure title={manual?localeText("details", "ui.DetailsPanel.a36e070a64395240"):localeText("details","queue.photos")} id="queue-items" initialOpen actions={<QueueListControls hideFinished={Boolean(taskQueue.hideFinished)} onToggle={onClearFinished}/>}><QueueCapsules/></Disclosure>
      <Disclosure title={localeText("details", "ui.DetailsPanel.d1771072821b57a4")} id="queue-details">      <div className="queue-current-photo" aria-label={localeText("details", "ui.DetailsPanel.b8794efd33c0c26a")}>
        {runningTask?<><CachedImage imagePath={runningTask.targetPath} originalPreviewUrl={runningTask.sourceSnapshot?.previewUrl} className="queue-current-photo__preview" width={80} height={80}/><div><strong title={runningTask.fileName}>{runningTask.fileName}</strong><span>{({preparing:localeText("details", "ui.DetailsPanel.7ab03426c6e083fc"),running:localeText("details", "ui.DetailsPanel.35e6d38ea8ac3244"),committing:localeText("details", "ui.DetailsPanel.747fde1978787a91")} as Record<string,string>)[runningTask.nativeStage??'running']} · {Math.min((runningTask.completedPasses??0)+1,taskQueue.config.passes??1)}/{taskQueue.config.passes??1}</span>{runningTask.currentStartedAtUnix?<small>{localeText("details", "ui.DetailsPanel.7cec0510c1cdffc3")}{' '}{Math.max(0,clock-runningTask.currentStartedAtUnix)} {' '}{localeText("details", "ui.DetailsPanel.8768a4a91a16f81d")}</small>:null}</div></>:<span className="queue-current-photo__empty">{native==='paused'?localeText("details", "ui.DetailsPanel.9ae361783f699888"):native==='completed'?localeText("details", "ui.DetailsPanel.bef8e09b9d212d56"):native==='stopped'?localeText("details", "ui.DetailsPanel.38c363027413167b"):localeText("details", "ui.DetailsPanel.c484b5b9dfad94ed")}</span>}
      </div>


      <div className="queue-counters-grid">
        <span><strong>{counters.total}</strong><small>{messages().details.total}</small></span>
        <span><strong>{counters.pending}</strong><small>{messages().details.pending}</small></span>
        <span><strong>{counters.running}</strong><small>{messages().details.running}</small></span>
        <span><strong>{counters.done}</strong><small>{messages().details.done}</small></span>
        <span><strong>{counters.failed}</strong><small>{messages().details.failed}</small></span>
        <span><strong>{counters.cancelled}</strong><small>{messages().details.cancelled}</small></span>
      </div>
      {counters.retryWaiting>0?<p role="status">{localeText("details", "ui.DetailsPanel.f2e6a15e049c3bcd")}{' '}{counters.retryWaiting}</p>:null}
</Disclosure>
      <QueueSettingsPanel queue={taskQueue}/>
      <QueueCostStatistics queue={taskQueue}/>
      <Disclosure title={localeText("details", "ui.DetailsPanel.594c22fef1ef7b31")} id="queue-history"><QueueHistory jobs={taskQueue.nativeJobs??[]} selectedId={taskQueue.nativeJobId}/></Disclosure>
    </section>
  );
}

function IndexTabContent({ indexDatabase, onRescanIndex }: { indexDatabase: IndexDatabaseState; onRescanIndex(): Promise<void> }) {
  useCurrentLocale();
  const dispatch=useAppDispatch(),[refreshing,setRefreshing]=useState(false),[refreshError,setRefreshError]=useState('');
  const refresh=async()=>{setRefreshing(true);setRefreshError('');try{const info=await indexService.getDatabaseInfo();dispatch({type:'index/initializeSucceeded',info});}catch(error){setRefreshError(String(error));}finally{setRefreshing(false);}};
  return (
    <section className="details-section details-index-section">
      <div data-tour-id="main.library.overview" data-help-id="index-library" className="section-header">
        <h3 className="section-title"><span>{localeText("details", "ui.DetailsPanel.028ceec6ca2ac3be")}</span><InfoHint label={localeText("details", "ui.DetailsPanel.9bf7c60ed62ddef5")}  message={{id:"library.index"}}/></h3>
        <Hint message={{id:"library.refresh-info"}}><button type="button" className="secondary-action library-refresh" aria-label={localeText("details", "ui.DetailsPanel.3764e1cb47397638")} title={localeText("details", "ui.DetailsPanel.f486e68246b11a94")} disabled={!indexDatabase.isReady||refreshing} onClick={()=>void refresh()}><Icon name="refresh"/>{localeText("details", "ui.DetailsPanel.16ba243a887d50bb")}</button></Hint>
      </div>

      <div className="library-overview">
        {refreshError?<span role="alert" className="danger">{refreshError}</span>:null}
        <div className="library-overview__counts"><span><strong>{indexDatabase.imageCount.toLocaleString(displayLocaleTag())}</strong> {' '}{localeText("details", "ui.DetailsPanel.113e650abdc10bec")}</span><span><strong>{indexDatabase.pathCount.toLocaleString(displayLocaleTag())}</strong> {' '}{localeText("details", "ui.DetailsPanel.9c6354a68e94ec05")}</span></div>
        <span className="library-overview__status">{indexDatabase.isReady?localeText("details", "ui.DetailsPanel.23343c2f4198e899"):indexDatabase.isInitializing?localeText("details", "ui.DetailsPanel.eb4bb68e67061327"):localeText("details", "ui.DetailsPanel.e09594b149c2b215")}</span>
        <span>{localeText("details", "ui.DetailsPanel.9ac11e5d1c214328")}{' '}{indexDatabase.lastNewImages} {' '}{localeText("details", "ui.DetailsPanel.40e24ecd9e53311c")}{' '}{indexDatabase.lastRelocatedImages} {' '}{localeText("details", "ui.DetailsPanel.4ee480674a3bbc20")}</span>
        <span>{indexDatabase.lastRescanAtUnix?localeText("details", "ui.DetailsPanel.9a1a12607d25d0b8")+formatUnixDateTime(indexDatabase.lastRescanAtUnix)+': '+indexDatabase.lastRescanCheckedPaths+localeText("details", "ui.DetailsPanel.ca18b11b7438fcba")+indexDatabase.lastMissingImages+localeText("details", "ui.DetailsPanel.507a4031c76428c3"):localeText("details", "ui.DetailsPanel.721e654a8fe58075")}</span>
        {indexDatabase.errorMessage?<span role="alert" className="danger">{nativeDisplayText(indexDatabase.errorMessage??'')}</span>:null}
      </div>

      <Disclosure title={localeText("details", "ui.DetailsPanel.d1e6695a1df23a31")} id="library-roots" hint={localeText("details", "ui.DetailsPanel.f326c4cee1589936")}><LibraryRootsPanel/></Disclosure>
      <Disclosure title={localeText("details", "ui.DetailsPanel.5e38601591e5994a")} id="index-maintenance" hint={localeText("details", "ui.DetailsPanel.43be54cd36355230")}><IndexMaintenancePanel indexDatabase={indexDatabase} onRescanIndex={onRescanIndex}/></Disclosure>
      <Disclosure title={localeText("details", "ui.DetailsPanel.397649944f98fb46")} id="index-duplicates" hint={localeText("details", "ui.DetailsPanel.4bd74634f5787452")}><DuplicatesPanel/></Disclosure>
      <IndexBackupPanel ready={indexDatabase.isReady}/>
      <Disclosure title={localeText('editor','cleanup.storageTitle')} id="project-storage"><ProjectStoragePanel/></Disclosure>
      <Disclosure title={localeText("details", "ui.DetailsPanel.af4975d348ebcda2")} id="index-cache" hint={localeText("details", "ui.DetailsPanel.2224ce36a0284946")}><ThumbnailCachePanel/></Disclosure>
      <Disclosure title={localeText("details", "ui.DetailsPanel.1c9f6e9d584c17b8")} id="index-technical" hint={localeText("details", "ui.DetailsPanel.542dddef3f66f9a4")}><p>{localeText("details", "ui.DetailsPanel.ffb5797e1eb4a419")}{' '}{indexDatabase.schemaVersion}</p><p className="copyable-text">{indexDatabase.databasePath}</p></Disclosure>
    </section>
  );
}

function IndexMaintenancePanel({ indexDatabase, onRescanIndex }: { indexDatabase: IndexDatabaseState; onRescanIndex(): Promise<void> }) {
  useCurrentLocale();
  const dispatch = useAppDispatch();
  const [maintenance, setMaintenance] = useState<IndexMaintenanceResult | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isWorking, setIsWorking] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [statusFilter, setStatusFilter] = useState<'missing' | 'all' | 'error' | 'relocated' | 'notProcessed'>('missing');
  const [offset,setOffset]=useState(0),[confirmation,setConfirmation]=useState<'remove'|'cleanup'|null>(null),request=useRef(0),confirmedIds=useRef<string[]>([]);
  useEffect(()=>()=>{request.current++;},[]);
  const [lastMessage, setLastMessage] = useState<string | null>(null);

  const loadMaintenance = async () => {
    const generation=++request.current;setSelectedIds([]);setIsLoading(true);
    setErrorMessage(null);
    try {
      const result = await indexService.getIndexMaintenance({ status: statusFilter, limit: 80, offset });
      if(generation!==request.current)return;if(offset>0&&offset>=result.totalMatchingImages){setOffset(Math.max(0,Math.floor((result.totalMatchingImages-1)/80)*80));return;}setMaintenance(result);
      setSelectedIds((current) => current.filter((id) => result.images.some((image) => image.imageId === id)));
    } catch (error) {
      if(generation===request.current)setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      if(generation===request.current)setIsLoading(false);
    }
  };

  useEffect(() => {
    if (!indexDatabase.isReady) return;
    void loadMaintenance();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [indexDatabase.isReady, indexDatabase.lastRescanAtUnix, indexDatabase.imageCount, statusFilter,offset]);

  const toggleSelected = (imageId: string) => {
    setSelectedIds((current) => current.includes(imageId)
      ? current.filter((id) => id !== imageId)
      : [...current, imageId]);
  };

  const selectAllVisible = () => {
    const visibleIds = maintenance?.images.map((image) => image.imageId) ?? [];
    setSelectedIds(visibleIds);
  };

  const clearSelection = () => setSelectedIds([]);

  const removeSelected = async () => {
    if (confirmedIds.current.length === 0) {
      setLastMessage(localeText("details", "ui.DetailsPanel.276f012859487096"));
      return;
    }

    setIsWorking(true);
    setErrorMessage(null);
    try {
      const result = await indexService.removeIndexImages({ imageIds: confirmedIds.current });
      dispatch({ type: 'index/countsUpdated', counts: { imageCount: result.imageCount, pathCount: result.pathCount, tagCount: result.tagCount } });
      setLastMessage(localeText("details", "ui.DetailsPanel.d5702a0e2082aab5", {"count": (result.removedImages)}));
      setSelectedIds([]);
      await loadMaintenance();
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsWorking(false);
    }
  };

  const cleanupGarbage = async () => {
    setIsWorking(true);
    setErrorMessage(null);
    try {
      const result = await indexService.cleanupIndexGarbage();
      dispatch({ type: 'index/countsUpdated', counts: { imageCount: result.imageCount, pathCount: result.pathCount, tagCount: result.tagCount } });
      setLastMessage(messages().details.cleanupCompleted(result.orphanTagsRemoved, result.oldSearchHistoryRemoved, result.staleFinishedTasksRemoved));
      await loadMaintenance();
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsWorking(false);
    }
  };

  const openLastKnownFolder = async (image: IndexMaintenanceImage) => {
    const parent = parentPathOf(image.path);
    if (!parent) {
      setLastMessage(localeText("details", "ui.DetailsPanel.50c79e494a644329"));
      return;
    }

    try {
      await fileOpenService.openExternalPath(parent);
      setLastMessage(localeText("details", "ui.DetailsPanel.e9a3497772a4eb40", {"value1": (parent)}));
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    }
  };

  const runBulkRescan = async () => {
    setIsWorking(true);
    setErrorMessage(null);
    try {
      await onRescanIndex();
      setLastMessage(localeText("details", "ui.DetailsPanel.6b2adb9e5552801f"));
      await loadMaintenance();
    } catch (error) {
      setErrorMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsWorking(false);
    }
  };

  return (
    <div className="index-maintenance-card">
      <div className="section-header">
        <strong>{messages().details.indexMaintenance}</strong>
        <Hint message={{id:"library.state-filter"}}><select className="compact-select" value={statusFilter} aria-label={localeText("details", "ui.DetailsPanel.d4ed77858e69411a")} onChange={(event) => {request.current++;setMaintenance(null);setSelectedIds([]);setOffset(0);setStatusFilter(event.target.value as typeof statusFilter);}}>
          <option value="missing">{messages().details.missing}</option>
          <option value="error">{nativeDisplayText(messages().explorer.statusLabels.error??'')}</option>
          <option value="relocated">{messages().explorer.statusLabels.relocated}</option>
          <option value="notProcessed">{messages().explorer.statusLabels.notProcessed}</option>
          <option value="all">{messages().details.allIndexedItems}</option>
        </select></Hint>
      </div>

      <div className="maintenance-summary-grid">
        <span><strong>{maintenance?.missingImageCount ?? indexDatabase.lastMissingImages}</strong><small>{messages().details.missing}</small></span>
        <span><strong>{maintenance?.orphanTagCount ?? 0}</strong><small>{messages().details.orphanTags}</small></span>
        <span><strong>{maintenance?.nonCurrentPathCount ?? 0}</strong><small>{messages().details.oldPaths}</small></span>
        <span><strong>{maintenance?.oldSearchHistoryCount ?? 0}</strong><small>{messages().details.oldSearches}</small></span>
      </div>

      <div className="maintenance-actions">
        <Hint message={{id:"library.records-refresh"}}><button type="button" className="secondary-action maintenance-refresh" aria-label={messages().details.reloadList} title={isLoading ? messages().details.loading : messages().details.reloadList} disabled={!indexDatabase.isReady || isLoading || isWorking} onClick={() => { void loadMaintenance(); }}><Icon name="refresh"/></button></Hint>
        <Hint message={{id:"library.check-files"}}><button type="button" className="secondary-action" disabled={!indexDatabase.isReady || isWorking} onClick={() => { void runBulkRescan(); }}>{localeText("details", "ui.DetailsPanel.e3b00c5a6f286b1f")}</button></Hint>
        <Hint message={{id:"library.clean-data"}}><button type="button" className="secondary-action" aria-label={localeText("details", "ui.DetailsPanel.f441d100b50236ee")} title={localeText("details", "ui.DetailsPanel.f64c706eede6fd5a")} disabled={!indexDatabase.isReady || isWorking} onClick={() => setConfirmation('cleanup')}>{localeText("details", "ui.DetailsPanel.87c3acf4e63ee25c")}</button></Hint>
      </div>

      <div className="maintenance-actions maintenance-actions--selection">
        <Hint message={{id:"library.select-page"}}><button type="button" className="secondary-action" aria-label={localeText("details", "ui.DetailsPanel.08edc4f8a5ffa166")} title={localeText("details", "ui.DetailsPanel.074967371580a2f3")} disabled={isLoading||isWorking||!maintenance?.images.length} onClick={selectAllVisible}>{localeText("details", "ui.DetailsPanel.24d1e08a7ca2ad1d")}</button></Hint>
        <Hint message={{id:"main.clear-selection"}}><button type="button" className="secondary-action" aria-label={localeText("details", "ui.DetailsPanel.b28494c11ca382ec")} disabled={selectedIds.length === 0} onClick={clearSelection}>{localeText("details", "ui.DetailsPanel.0bf2bb7dab1d36ea")}</button></Hint>
        <Hint message={{id:"library.remove-index"}}><button type="button" className="secondary-action danger-action" disabled={selectedIds.length === 0 || isLoading || isWorking} onClick={() => {confirmedIds.current=[...selectedIds];setConfirmation('remove');}}>{messages().details.removeFromIndex}</button></Hint>
      </div>

      {lastMessage ? <p className="metadata-edit-note">{lastMessage}</p> : null}
      {errorMessage ? <p className="danger">{messages().details.maintenanceError}: {errorMessage}</p> : null}
      {maintenance ? <p className="metadata-edit-note">{messages().details.recordsShown(maintenance.returnedImages, maintenance.totalMatchingImages)} {messages().details.indexRemovalSafety}</p> : null}

      {maintenance&&maintenance.totalMatchingImages>80?<nav className="details-page-controls" aria-label={localeText("details", "ui.DetailsPanel.e2669727ec2eae75")}><Hint message={{id:"common.page-prev",args:{список:localeText("details", "ui.DetailsPanel.5c4d5d33cb0c4098")}}}><button type="button" disabled={isLoading||isWorking||offset===0} onClick={()=>setOffset(v=>Math.max(0,v-80))}>{localeText("details", "ui.DetailsPanel.0e61061175d96e21")}</button></Hint><span>{offset+1}–{offset+maintenance.returnedImages} {' '}{localeText("details", "ui.DetailsPanel.d334809c448daf34")}{' '}{maintenance.totalMatchingImages}</span><Hint message={{id:"common.page-next",args:{список:localeText("details", "ui.DetailsPanel.5c4d5d33cb0c4098")}}}><button type="button" disabled={isLoading||isWorking||offset+80>=maintenance.totalMatchingImages} onClick={()=>setOffset(v=>v+80)}>{localeText("details", "ui.DetailsPanel.1c145118f2e9448e")}</button></Hint></nav>:null}
      {statusFilter==='all'?<small>{localeText("details", "ui.DetailsPanel.c9be1477a24703ca")}</small>:null}
      {selectedIds.length?<small>{localeText("details", "ui.DetailsPanel.c60d6badcce4cb26")}{' '}{selectedIds.length}</small>:null}
      {confirmation?<EditorModal title={confirmation==='remove'?localeText("details", "ui.DetailsPanel.f687d582d77fbeba"):localeText("details", "ui.DetailsPanel.b1ed97150d436e20")} busy={isWorking} onClose={()=>setConfirmation(null)}><p>{confirmation==='remove'?localeText("details", "ui.DetailsPanel.815d3b23f77cb5b8", {"value1": (confirmedIds.current.length)}):localeText("details", "ui.DetailsPanel.6d7f853beac1713f")}</p><footer><Hint message={{id:"common.cancel-draft"}}><button type="button" disabled={isWorking} onClick={()=>setConfirmation(null)}>{localeText("details", "ui.DetailsPanel.8c075ebaaf3bf1de")}</button></Hint><Hint message={{id:"library.confirm-clean"}}><button type="button" disabled={isWorking} onClick={()=>void (confirmation==='remove'?removeSelected():cleanupGarbage()).then(()=>setConfirmation(null))}>{localeText("details", "ui.DetailsPanel.2da3e74015832250")}</button></Hint></footer></EditorModal>:null}
      <div className="maintenance-list">
        {maintenance?.images.length === 0 ? <p className="empty-details">{messages().details.noRecordsForFilter}</p> : null}
        {maintenance?.images.map((image) => (
          <article key={image.imageId} className={`maintenance-row maintenance-row--${image.status}`}>
            <Hint message={{id:"library.record-check"}}><label className="maintenance-row__check">
              <input type="checkbox" disabled={isLoading||isWorking} checked={selectedIds.includes(image.imageId)} onChange={() => toggleSelected(image.imageId)} />
            </label></Hint>
            <div className="maintenance-row__body">
              <strong title={image.fileName}>{image.fileName}</strong>
              <span title={image.path}>{image.path || messages().details.noCurrentPathRecorded}</span>
              <small className="maintenance-row__reason">{image.status==='missing'?localeText("details", "ui.DetailsPanel.ff523c48bb106ade"):image.status==='relocated'?localeText("details", "ui.DetailsPanel.900542aab47d93b1"):image.status==='error'?localeText("details", "ui.DetailsPanel.fe5c997a1428defe"):image.status==='processed'?localeText("details", "ui.DetailsPanel.55a2f76974263ba3"):localeText("details", "ui.DetailsPanel.d21c679519187f0c")}</small>
              <small title={localeText("details", "ui.DetailsPanel.1d3a4c9d03b87fe5", {"value1": (messages().details.lastSeen), "value2": (image.lastSeenAtUnix ? formatUnixDateTime(image.lastSeenAtUnix) : messages().common.unknown), "value3": (image.missingAtUnix ? ` · ${messages().details.missingSince} ${formatUnixDateTime(image.missingAtUnix)}` : '')})}>{messages().details.lastSeen} {image.lastSeenAtUnix ? formatUnixDateTime(image.lastSeenAtUnix) : messages().common.unknown}</small>
              {image.tags.length > 0 ? <small>{messages().details.tags}: {image.tags.slice(0, 6).join(', ')}</small> : null}
            </div>
            <Hint message={{id:image.status==='missing'?'library.last-folder':'common.reveal-file'}}><button type="button" className="secondary-action maintenance-row__folder" aria-label={localeText("details", "ui.DetailsPanel.b5e44cf985ead910", {"value1": (image.status==='missing'?localeText("details", "ui.DetailsPanel.be72a3c0ae8b0cb9"):messages().details.openFolder), "value2": (image.fileName)})} title={image.status==='missing'?localeText("details", "ui.DetailsPanel.9b8e1a7224c17a5d"):localeText("details", "ui.DetailsPanel.6a57ad3d34d2ef6a")} disabled={!image.path} onClick={() => { void openLastKnownFolder(image); }}><Icon name="folder"/></button></Hint>
          </article>
        ))}
      </div>
    </div>
  );
}

function parentPathOf(path: string): string | null {
  const normalized = path.trim();
  if (!normalized) return null;
  const lastSlash = Math.max(normalized.lastIndexOf('/'), normalized.lastIndexOf('\\'));
  if (lastSlash <= 0) return null;
  return normalized.slice(0, lastSlash);
}

function JournalTabContent({ selectedItem }: { selectedItem: ImageItem | null }) {
  useCurrentLocale();
  const [entries, setEntries] = useState<ActionJournalEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [loadedAtUnix, setLoadedAtUnix] = useState<number | null>(null);
  const [scope,setScope]=useState<'photo'|'all'>('photo');
  const [reload,setReload]=useState(0),[offset,setOffset]=useState(0),[total,setTotal]=useState(0),identity=useRef('');
  const selectedIndexedImageId = scope==='photo' ? selectedItem?.indexedImageId ?? null : null;

  useEffect(()=>setOffset(0),[selectedIndexedImageId]);
  useEffect(() => {
    let isCancelled = false;
    setIsLoading(true);
    setErrorMessage(null);
    const key=`${selectedIndexedImageId??'all'}:${offset}`;if(identity.current!==key){identity.current=key;setEntries([]);}

    actionLogService
      .getActionJournal({
        imageId: selectedIndexedImageId,
        limit: 80,
        offset,
        includeSearchHistory: selectedIndexedImageId === null,
      })
      .then((result) => {
        if (isCancelled) return;
        setEntries(result.entries);setTotal(result.totalCount);
        setLoadedAtUnix(result.loadedAtUnix);
      })
      .catch((error: unknown) => {
        if (isCancelled) return;
        setErrorMessage(error instanceof Error ? error.message : String(error));
      })
      .finally(() => {
        if (!isCancelled) setIsLoading(false);
      });

    return () => {
      isCancelled = true;
    };
  }, [selectedIndexedImageId,reload,offset]);

  const title = localeText("details", "ui.DetailsPanel.d22383f9f2088ef7");

  return (
    <section data-tour-id="main.journal.overview" data-help-id="journal-panel" className="details-section journal-section">
      <div className="section-header">
        <h3 className="section-title"><span>{title}</span><InfoHint label={localeText("details", "ui.DetailsPanel.0f59a71b55ab690e")}  message={{id:"details.tab-journal"}}/></h3>
        {loadedAtUnix?<span className="journal-loaded-time" title={formatUnixDateTime(loadedAtUnix)}>{new Date(loadedAtUnix*1000).toLocaleTimeString(displayLocaleTag())}</span>:null}
        <Hint message={{id:"journal.scope"}}><select aria-label={localeText("details", "ui.DetailsPanel.e994b13e6438970c")} value={selectedIndexedImageId?'photo':'all'} onChange={event=>setScope(event.target.value as 'photo'|'all')}><option value="photo" disabled={!selectedItem?.indexedImageId}>{localeText("details", "ui.DetailsPanel.0090cea33bae08cd")}</option><option value="all">{localeText("details", "ui.DetailsPanel.5e34df7537502535")}</option></select></Hint>
        <Hint message={{id:"journal.refresh"}}><button type="button" className="icon-button" aria-label={localeText("details", "ui.DetailsPanel.c20747072ae41545")} disabled={isLoading} onClick={()=>setReload(value=>value+1)}><Icon name="refresh"/></button></Hint>
      </div>

      {errorMessage ? <p className="danger">{messages().details.journalLoadFailed}: {errorMessage}</p> : null}
      {!isLoading && entries.length === 0 ? <p className="empty-details">{messages().details.noJournalEntriesYet}</p> : null}

      <div className="journal-entry-list" aria-busy={isLoading}>
        {entries.map((entry) => <JournalEntryRow key={entry.id} entry={entry} scopedImageId={selectedIndexedImageId} />)}
      </div>
      {total>80?<nav className="details-page-controls" aria-label={localeText("details", "ui.DetailsPanel.0b95331d6b4830e0")}><Hint message={{id:"common.page-prev",args:{список:localeText("details", "ui.DetailsPanel.c92e15d329c389e0")}}}><button type="button" disabled={isLoading||offset===0} onClick={()=>setOffset(v=>Math.max(0,v-80))}>{localeText("details", "ui.DetailsPanel.4aa08898c1f5fc6e")}</button></Hint><span>{offset+1}–{Math.min(total,offset+entries.length)} {' '}{localeText("details", "ui.DetailsPanel.27d50fca79ec1189")}{' '}{total}</span><Hint message={{id:"common.page-next",args:{список:localeText("details", "ui.DetailsPanel.c92e15d329c389e0")}}}><button type="button" disabled={isLoading||offset+80>=total} onClick={()=>setOffset(v=>v+80)}>{localeText("details", "ui.DetailsPanel.9a61e030f10540d8")}</button></Hint></nav>:null}
    </section>
  );
}

function JournalEntryRow({ entry,scopedImageId }: { entry: ActionJournalEntry;scopedImageId:string|null }) {
  useCurrentLocale();
  const [expanded,setExpanded]=useState(false);
  const parsedNewValue = formatJournalValue(entry.newValue);
  const parsedOldValue = formatJournalValue(entry.oldValue);
  const {title,summary,files,renames,changes,notes,metadata,path}=journalPresentation(entry,scopedImageId);
  const icon=entry.actionType.includes('favorite')?'star':entry.actionType.includes('rename')?'edit':entry.actionType.includes('path')?'folder':entry.actionType.includes('error')?'info':'check';

  return (
    <article className={`journal-entry journal-entry--${entry.actionType.replace(/[^a-z0-9_-]/gi, '-')}`}>
      <div className="journal-entry__header">
        <time title={formatUnixDateTime(entry.createdAtUnix)}>{new Date(entry.createdAtUnix*1000).toLocaleTimeString(displayLocaleTag(), {hour:'2-digit',minute:'2-digit'})}</time>
        <span className="journal-entry__icon"><Icon name={icon}/></span>
        <strong className="journal-entry__type">{title}</strong>
        <Hint message={{id:"journal.event"}}><button type="button" className="icon-button" aria-label={localeText("details", "ui.DetailsPanel.50d139e5b82c58b0", {"value1": (entry.id)})} aria-expanded={expanded} onClick={()=>setExpanded(value=>!value)}><Icon name={expanded?'up':'down'}/></button></Hint>
      </div>
      {summary?<p className="journal-entry__summary" title={path??undefined}>{summary}</p>:null}
      {expanded?<div className="journal-entry__changes copyable-text">
        {files.length?<div>{files.map((file,index)=><p key={file.imageId} title={file.path}><strong>{file.fileName??localeText("details", "ui.DetailsPanel.154566a317938e6d", {"value1": (index+1)})}</strong>: {(file.visible??[]).join(', ')||localeText("details", "ui.DetailsPanel.f0ff3941b838402d")} → {(file.after??[]).join(', ')||localeText("details", "ui.DetailsPanel.f0ff3941b838402d")}</p>)}</div>:null}
        {renames.map((file,index)=><p key={`${file.imageId??''}:${index}`} title={localeText("details", "ui.DetailsPanel.c0c28cb8e1a61d52", {"value1": (file.oldPath), "value2": (file.newPath)})}><strong>{journalFileName(file.oldPath)}</strong> → {journalFileName(file.newPath)}</p>)}
        {changes.map(change=><p key={change.label}><strong>{change.label}</strong>: {change.before} → {change.after}</p>)}
        {notes.map(note=><p key={note}>{note}</p>)}
        {!files.length&&!renames.length&&!metadata?<p>{entry.message}</p>:null}
        {path&&!renames.length ? <p>{messages().details.journalPath}: {path}</p> : null}
        {parsedOldValue||parsedNewValue||renames.length?<details className="journal-entry__technical"><Hint message={{id:"common.expand",args:{раздел:localeText("details", "ui.DetailsPanel.087d3f7c3f20ce1d")}}}><summary>{localeText("details", "ui.DetailsPanel.11c78cc4aac1099f")}</summary></Hint>{parsedOldValue?<div><strong>{messages().details.oldValue}</strong><pre>{parsedOldValue}</pre></div>:null}{parsedNewValue?<div><strong>{messages().details.newValue}</strong><pre>{parsedNewValue}</pre></div>:null}{renames.length?<><pre>{entry.message}</pre>{entry.relatedRenames?<pre>{JSON.stringify(entry.relatedRenames,null,2)}</pre>:null}</>:null}</details>:null}
      </div>:null}
    </article>
  );
}

function formatJournalValue(value?: string | null): string | null {
  if (!value) return null;
  try {
    return JSON.stringify(JSON.parse(value), null, 2);
  } catch {
    return value;
  }
}

function formatUnixDateTime(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return messages().details.unknownTime;
  return new Date(value * 1000).toLocaleString(displayLocaleTag());
}

function parseTags(rawText: string): string[] {
  const result: string[] = [];
  const seen = new Set<string>();

  for (const rawTag of rawText.split(/[;,\n]/)) {
    const tag = rawTag.trim().replace(/^#+/, '').toLowerCase();
    if (!tag || seen.has(tag)) continue;
    seen.add(tag);
    result.push(tag);
  }

  return result;
}

function normalizeComparableTags(tags: string[]): string[] {
  return parseTags(tags.join(','));
}

interface PanelTabsProps {
  activeTab: DetailsTab;
  onTabChange(tab: DetailsTab): void;
}

function PanelTabs({ activeTab, onTabChange }: PanelTabsProps) {
  useCurrentLocale();
  const tabs: Array<{ id: DetailsTab; label: string }> = [
    { id: 'details', label: messages().details.detailsTab },
    { id: 'aiDescription', label: localeText("details", "ui.DetailsPanel.8dba03014727e7db") },
    { id: 'tags', label: messages().details.tagsTab },
    { id: 'tasks', label: messages().details.tasksTab },
    { id: 'index', label: localeText("details", "ui.DetailsPanel.62660699ee33a3ed") },
    { id: 'journal', label: messages().details.journalTab },
  ];

  return (
    <div data-tour-id="main.inspector.tabs" data-help-id="inspector-tabs" className="details-tabs" role="tablist" aria-label={messages().details.detailsTab}>
      {tabs.map((tab) => (
        <Hint key={tab.id} message={{id:({details:'details.tab-info',aiDescription:'details.tab-description',tags:'details.tab-tags',tasks:'details.tab-tasks',index:'details.tab-library',journal:'details.tab-journal'} as const)[tab.id]}}><button
          key={tab.id}
          type="button"
          className={activeTab === tab.id ? 'active' : undefined}
          role="tab"
          aria-selected={activeTab === tab.id}
          onClick={() => onTabChange(tab.id)}
        >
          {tab.label}
        </button></Hint>
      ))}
    </div>
  );
}
