import type {ExplorerItem} from '../../modules/explorer/types';
import type {AppState} from '../types';
import {explorerItemComparator} from './appSelectors';
/** Merge only new/changed sort keys; unchanged metadata never reshuffles the old run. */
export function reconcileExplorerOrder(items:ExplorerItem[],oldOrder:string[],snapshot:Record<string,ExplorerItem>|undefined,mode:AppState['sortMode'],group:AppState['folderGrouping']){
 const compare=explorerItemComparator(mode,group),byId=new Map(items.map(item=>[item.id,item])),retained:ExplorerItem[]=[],changed:ExplorerItem[]=[];
 const oldIds=new Set(oldOrder);
 for(const id of oldOrder){const item=byId.get(id),old=snapshot?.[id];if(item&&old&&compare(old,item)===0)retained.push(item);else if(item)changed.push(item);}
 for(const item of items)if(!oldIds.has(item.id))changed.push(item);
 changed.sort(compare);const order:string[]=[];let a=0,b=0;
 while(a<retained.length||b<changed.length){if(b>=changed.length||a<retained.length&&compare(retained[a],changed[b])<=0)order.push(retained[a++].id);else order.push(changed[b++].id);}
 return {explorerDisplayOrder:order.length===oldOrder.length&&order.every((id,i)=>id===oldOrder[i])?oldOrder:order,explorerSortSnapshot:Object.fromEntries(items.map(item=>[item.id,item]))};
}
