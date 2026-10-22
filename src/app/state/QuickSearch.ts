import type {AppState} from '../types';
import {defaultSearchFields} from '../../modules/search/SearchFieldsContract';
export type SearchSelection=Pick<AppState,'selectedItemId'|'selectedItemIds'|'selectionAnchorItemId'|'inspectedImage'>;
export interface QuickSearchState {query:string;active:boolean;composing:boolean;search:AppState['search'];baseSelection?:SearchSelection;dataRevision?:number}
export function initialQuickSearch(state:AppState):QuickSearchState{
 return {query:'',active:false,composing:false,search:{...state.search,fields:[...defaultSearchFields],rootPath:'',collectionId:'',statusFilter:'all',tagFilterText:'',tagMode:'any',dateFrom:'',dateTo:'',favoriteOnly:false,untaggedOnly:false,sortOrder:'relevance',appliedSortOrder:'relevance',results:[],resultCount:0,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,isSearching:false,isLoadingPage:false,errorMessage:null,pageError:null,lastSearchedAtUnix:null,lastQuery:'',statusCounts:undefined,lastAppliedTags:[],lastAppliedStatuses:[],lastAppliedDateFromUnix:null,lastAppliedDateToUnix:null,lastAppliedFavoriteOnly:false}};
}
export function selectionOf(s:AppState):SearchSelection{return {selectedItemId:s.selectedItemId,selectedItemIds:s.selectedItemIds,selectionAnchorItemId:s.selectionAnchorItemId,inspectedImage:s.inspectedImage};}
/** View only: the underlying advanced SearchState remains owned by its workspace. */
export function quickSearchView(s:AppState):AppState{
 if(!s.quickSearch?.active||s.underlyingSearch)return s;
 return {...s,underlyingSearch:{search:s.search,query:s.searchQuery},search:s.quickSearch.search,searchQuery:s.quickSearch.query};
}
export function isQueryView(s:AppState):boolean{return Boolean(s.quickSearch?.active)||s.activeWorkspace!=='explorer';}
