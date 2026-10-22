import {invoke} from "../../shared/tauri/invoke";
export interface BatchRenameRules {
 commonName:string|null; prefix:string; suffix:string; numbering:boolean; start:number; digits:number; step:number;
 position:'before'|'after'; order:'current'|'name'|'modified'; perFolder:boolean;
 separator:string|null;
}
export interface BatchRenameRow {
 oldPath:string;newPath:string;oldName:string;newName:string;status:'ready'|'unchanged'|'conflict'|'error';message:string;
 imageId:string|null;sizeBytes:number;modifiedAtUnix:number;
}
export interface BatchRenamePlan {id:string;state:string;createdAtUnix:number;rules:BatchRenameRules;rows:BatchRenameRow[];error:string|null;reverse:boolean;completedCount:number;remainingCount:number}
export interface BatchRenameHistory {id:string;state:string;createdAtUnix:number;count:number;error:string|null}
export const defaultBatchRenameRules:BatchRenameRules={commonName:null,prefix:'',suffix:'',numbering:true,start:1,digits:3,step:1,position:'after',order:'current',perFolder:false,separator:'_'};
export const batchRenameService={
 preview:(paths:string[],includeSubfolders:boolean,rules:BatchRenameRules)=>invoke<BatchRenamePlan>('preview_batch_rename',{input:{paths,includeSubfolders,rules}}),
 names:(id:string,rules:BatchRenameRules)=>invoke<BatchRenamePlan>('preview_batch_rename_names',{id,rules}),
 command:(id:string,action:'apply'|'continue'|'rollback'|'inspect',rules?:BatchRenameRules)=>invoke<BatchRenamePlan>('batch_rename_command',{id,action,rules}),
 history:()=>invoke<BatchRenameHistory[]>('get_batch_rename_history'),
 selectionPaths:(queryId:string,excludedPaths:string[])=>invoke<string[]>('get_batch_rename_selection_paths',{queryId,excludedPaths}),
};
export function openBatchRename(paths?:string[]){window.dispatchEvent(new CustomEvent('smart-photo-indexer:open-batch-rename',{detail:{paths}}));}
