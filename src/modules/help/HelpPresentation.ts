import type{SettingsTab}from'../../shared/SettingsTabs';
import {useState,useSyncExternalStore} from 'react';
import type {DetailsTab,WorkspaceMode} from '../../app/types';

/** Only visual state. Never pass these values to business actions or persist them. */
export interface TourPresentation {
 editorMenu?:boolean;historyTree?:boolean;brushField?:boolean;toolsMenu?:boolean;settingsOpen?:boolean;settingsTab?:SettingsTab;workspace?:WorkspaceMode;detailsTab?:DetailsTab;rightPanel?:boolean;leftPanel?:boolean;
 recent?:boolean;searchPane?:'filters'|'saved';helpDrawer?:boolean;disclosures?:string[];
 inspectorTab?:'edit'|'tasks'|'sources';chromeVisible?:boolean;pauseTimerOnly?:boolean;
}
interface PresentationState {open:boolean;stepId:string|null;view:TourPresentation}
const empty:TourPresentation={};
let state:PresentationState={open:false,stepId:null,view:empty};
const listeners=new Set<()=>void>();
function update(next:PresentationState){state=next;for(const listener of listeners)listener();}
export function setHelpOpen(open:boolean){update({open,stepId:null,view:empty});}
export function setHelpStep(stepId:string|null,view:TourPresentation=empty){update({open:state.open,stepId,view});}
export function helpPresentation(){return state;}
export function useHelpPresentation(){return useSyncExternalStore(listener=>{listeners.add(listener);return()=>listeners.delete(listener);},()=>state,()=>state);}
export function useHelpDisclosure(id:string|undefined){const help=useHelpPresentation();return Boolean(id&&help.view.disclosures?.includes(id));}
export function useTourDisclosure(id:string,initialOpen=false){
 const help=useHelpPresentation(),[open,setOpen]=useState(initialOpen);
 return {open:open||Boolean(help.view.disclosures?.includes(id)),onToggle:(event:{currentTarget:HTMLDetailsElement})=>{if(!help.open)setOpen(event.currentTarget.open);}};
}
