import {text as localeText} from "../../shared/i18n/Runtime";
import type {AppState,WorkspaceMode} from '../types';

export function switchWorkspace(state:AppState,mode:WorkspaceMode):AppState{
  if(mode===state.activeWorkspace)return state;
  const contexts={...state.workspaceContexts,[state.activeWorkspace]:{
    search:{...state.search,isSearching:false,isLoadingPage:false},query:state.searchQuery,selectedItemId:state.selectedItemId,
    selectedItemIds:state.selectedItemIds,selectionAnchorItemId:state.selectionAnchorItemId,
  }};
  const saved=contexts[mode];
  const search=(saved?.search)??(mode==='favorites'||mode==='ready'||mode==='collections'?{
    ...state.search,queryId:undefined,lastInput:undefined,selectionSnapshot:undefined,results:[],resultCount:0,
    isSearching:false,isLoadingPage:false,errorMessage:null,lastQuery:'',statusFilter:mode==='ready'?'processed' as const:'all' as const,
    favoriteOnly:mode==='favorites',lastAppliedFavoriteOnly:false,untaggedOnly:false,collectionId:'',rootPath:'',
    tagFilterText:'',lastAppliedTags:[],lastAppliedStatuses:[],dateFrom:'',dateTo:'',sortOrder:'nameAsc',appliedSortOrder:'nameAsc',statusCounts:undefined,needsRefresh:false,
  }:state.search);
  return {...state,workspaceContexts:contexts,activeWorkspace:mode,search,
    searchQuery:saved?.query??((mode==='favorites'||mode==='ready'||mode==='collections')?'':state.searchQuery),
    selectedItemId:saved?.selectedItemId??null,selectedItemIds:saved?.selectedItemIds??[],selectionAnchorItemId:saved?.selectionAnchorItemId??null,
    contextMenu:{...state.contextMenu,isOpen:false},statusMessage:({explorer:localeText("main", "ui.WorkspaceContexts.791daa66385f5fd3"),search:localeText("main", "ui.WorkspaceContexts.1c4f201b9aa84509"),favorites:localeText("main", "ui.WorkspaceContexts.096b736845606ca8"),ready:localeText("main", "ui.WorkspaceContexts.77f7958f2f5fae5e"),collections:localeText("main", "ui.WorkspaceContexts.e203baf015887f17")})[mode],
  };
}
