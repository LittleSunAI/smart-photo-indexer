import type {AppState} from '../types';
import type {TourPresentation} from '../../modules/help/HelpPresentation';

/** Read-only projection of live data; no action, owned snapshot, fetch or restore. */
const cache=new WeakMap<AppState,{view:TourPresentation;result:AppState}>();
export function workspacePresentation(state:AppState,view:TourPresentation):AppState{
 const previous=cache.get(state);if(previous?.view===view)return previous.result;
 const result=project(state,view);cache.set(state,{view,result});return result;
}
function project(state:AppState,view:TourPresentation):AppState{
 const mode=view.workspace;
 if(!mode)return state;
 const original=state.underlyingSearch??{search:state.search,query:state.searchQuery};
 const saved=state.workspaceContexts?.[mode];
 const same=mode===state.activeWorkspace;
 const search=same?original.search:saved?.search??{
  ...original.search,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,results:[],resultCount:0,
  isSearching:false,isLoadingPage:false,errorMessage:null,pageError:null,lastSearchedAtUnix:null,lastQuery:'',
  collectionId:'',favoriteOnly:mode==='favorites',statusFilter:mode==='ready'?'processed' as const:'all' as const,
  lastAppliedFavoriteOnly:false,lastAppliedTags:[],lastAppliedStatuses:[],statusCounts:undefined,needsRefresh:false,
 };
 const selection=same?(state.quickSearch?.active?state.quickSearch.baseSelection:state):saved;
 return {...state,activeWorkspace:mode,search,searchQuery:same?original.query:saved?.query??'',underlyingSearch:undefined,
  quickSearch:state.quickSearch?{...state.quickSearch,active:false}:undefined,
  selectedItemId:selection?.selectedItemId??null,selectedItemIds:selection?.selectedItemIds??[],selectionAnchorItemId:selection?.selectionAnchorItemId??null,
  contextMenu:{...state.contextMenu,isOpen:false},
 };
}
