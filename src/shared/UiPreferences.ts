import{settingsTabs,type SettingsTab}from'./SettingsTabs';
import {normalizeLocale,type Locale} from './i18n/Locale';
export interface UiPreferences {version:1;lastSettingsTab?:SettingsTab;editorProjectSeed?:'fresh';locale?:Locale;theme:'light'|'dark';editorAutosave:boolean;viewerOnlyPhoto:boolean;hoverHintsEnabled:boolean;thumbnailFit:'cover'|'contain';editorMarkupHeight:number|null;editorHistoryHeight:number|null;detailsPreviewHeight:number|null;queueHideFinished?:boolean;queueManualTarget?:string|null;collectionTarget?:string|null}
export const UI_PREFERENCES_KEY='smart-photo-indexer.ui.v1',UI_PREFERENCES_EVENT='smart-photo-indexer:ui-preferences';
let queueTargetVersion=0;
export const manualQueueTargetVersion=()=>queueTargetVersion;
const defaults:UiPreferences={version:1,locale:'ru',theme:'light',editorAutosave:true,viewerOnlyPhoto:false,hoverHintsEnabled:true,thumbnailFit:'cover',editorMarkupHeight:null,editorHistoryHeight:null,detailsPreviewHeight:null};
export function loadUiPreferences():UiPreferences {
 try{const raw=JSON.parse(localStorage.getItem(UI_PREFERENCES_KEY)??'null');if(!raw){const legacy=JSON.parse(localStorage.getItem('smart-photo-indexer.settings.v1')??'null');return {...defaults,theme:legacy?.theme==='dark'?'dark':'light'};}if(raw.version!==1)return {...defaults};return {version:1,...(raw.editorProjectSeed==='fresh'?{editorProjectSeed:'fresh' as const}:{}),locale:normalizeLocale(raw.locale),lastSettingsTab:settingsTabs.includes(raw.lastSettingsTab)?raw.lastSettingsTab:undefined,theme:raw.theme==='dark'?'dark':'light',editorAutosave:raw.editorAutosave!==false,viewerOnlyPhoto:raw.viewerOnlyPhoto===true,hoverHintsEnabled:raw.hoverHintsEnabled!==false,thumbnailFit:raw.thumbnailFit==='contain'?'contain':'cover',queueHideFinished:raw.queueHideFinished===true,collectionTarget:typeof raw.collectionTarget==='string'&&raw.collectionTarget.length<=200?raw.collectionTarget:null,queueManualTarget:typeof raw.queueManualTarget==='string'&&raw.queueManualTarget.trim().length>0&&raw.queueManualTarget.length<=200?raw.queueManualTarget:null,detailsPreviewHeight:typeof raw.detailsPreviewHeight==='number'&&Number.isFinite(raw.detailsPreviewHeight)?Math.max(96,Math.min(640,Math.round(raw.detailsPreviewHeight))):null,editorHistoryHeight:typeof raw.editorHistoryHeight==='number'&&Number.isFinite(raw.editorHistoryHeight)?Math.max(96,Math.min(400,Math.round(raw.editorHistoryHeight))):null,editorMarkupHeight:typeof raw.editorMarkupHeight==='number'&&Number.isFinite(raw.editorMarkupHeight)?Math.max(36,Math.min(1200,Math.round(raw.editorMarkupHeight))):null};}catch{return {...defaults};}
}
/** Write fresh preferences first. A denied write must not look like a successful setting change. */
export function saveUiPreferences(change:Partial<Omit<UiPreferences,'version'>>):UiPreferences {
 let prior:unknown;try{prior=JSON.parse(localStorage.getItem(UI_PREFERENCES_KEY)??'null');}catch{}
 const fresh=!(prior&&typeof prior==='object'&&(prior as {version?:number}).version===1);
 const next={...loadUiPreferences(),...(fresh?{editorProjectSeed:'fresh' as const}:{}),...change,version:1 as const};localStorage.setItem(UI_PREFERENCES_KEY,JSON.stringify(next));if(Object.prototype.hasOwnProperty.call(change,'queueManualTarget'))queueTargetVersion++;window.dispatchEvent(new Event(UI_PREFERENCES_EVENT));return next;
}

/** Storage failures remain observable to the locale controller; corrupt/old values safely mean RU. */
export function readUiLocale():Locale{
 const value=localStorage.getItem(UI_PREFERENCES_KEY);if(!value)return 'ru';
 let raw;try{raw=JSON.parse(value);}catch{return 'ru';}
 return raw?.version===1?normalizeLocale(raw.locale):'ru';
}
