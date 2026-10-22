import {normalizeSearchOrder} from './SearchOrder';
import {defaultSearchFields} from './SearchFieldsContract';
import type {SearchState,IndexSearchInput} from '../index/types';
export function searchDraftInput(query:string,s:SearchState):IndexSearchInput{
 const tags=[...new Set(s.tagFilterText.split(/[\n,;]+/).map(x=>x.trim().replace(/^#+/,'').toLowerCase()).filter(Boolean))];
 const civilNext=(v:string)=>{if(!v)return undefined;const d=new Date(`${v}T12:00:00`);if(!Number.isFinite(d.getTime()))return undefined;d.setDate(d.getDate()+1);return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;};
 const unix=(v:string,end=false)=>{if(!v)return null;const d=new Date(`${v}T00:00:00`);if(end)d.setDate(d.getDate()+1);return Number.isFinite(d.getTime())?Math.floor(d.getTime()/1000):null;};
 return {fields:s.fields??defaultSearchFields,query:query.trim(),statuses:s.statusFilter==='all'?[]:[s.statusFilter],tags,tagMode:s.tagMode,dateFromUnix:s.dateField==='taken'?undefined:unix(s.dateFrom),dateToUnix:s.dateField==='taken'?undefined:unix(s.dateTo,true),favoriteOnly:s.favoriteOnly,untaggedOnly:s.untaggedOnly===true,expandTerms:s.expandTerms,collectionId:s.collectionId||undefined,rootPath:s.rootPath||undefined,sortOrder:normalizeSearchOrder(s.sortOrder),dateField:s.dateField,dateFromLocal:s.dateField==='taken'?s.dateFrom||undefined:undefined,dateToLocal:s.dateField==='taken'?civilNext(s.dateTo):undefined,dateToExclusive:true,limit:200,offset:0};
}
export function searchInputSignature(input:IndexSearchInput):string{
 const {queryId:_queryId,limit:_limit,offset:_offset,...fields}=input;
 return JSON.stringify(Object.fromEntries(Object.entries(fields).filter(([,v])=>v!==undefined).sort(([a],[b])=>a.localeCompare(b))));
}
