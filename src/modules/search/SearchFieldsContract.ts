import {text as localeText} from "../../shared/i18n/Runtime";
export function searchFieldOptions(){return [['description',localeText("search", "ui.SearchFieldsContract.8c2ad07c56f1fce0")],['tags',localeText("search", "ui.SearchFieldsContract.2e2682710039af64")],['fileName',localeText("search", "ui.SearchFieldsContract.6815d1f6cd30971b")],['path',localeText("search", "ui.SearchFieldsContract.0c5a2389be9c2b03")]] as const;}
export type SearchField=ReturnType<typeof searchFieldOptions>[number][0];
export const defaultSearchFields:SearchField[]=['description','tags'];
export const legacySearchFields:SearchField[]=['description','tags','fileName','path'];
export function normalizeSearchFields(value:unknown,fallback=defaultSearchFields):SearchField[]{return Array.isArray(value)?searchFieldOptions().map(([key])=>key).filter(key=>value.includes(key)):fallback;}
