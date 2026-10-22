import legacy from './legacy-step-map.json';
import type {HelpContext,HelpFeature,HelpGroup} from './HelpRegistry';
export const HELP_PROGRESS_KEY='smart-photo-indexer.help.v2';
export interface HelpProgress {version:2;tab:'about'|'tour'|'reference';articleId:string;steps:Partial<Record<HelpContext,{stepId:string;scope:string}>>;stale?:boolean}
export function readHelpProgress(features:HelpFeature[],groups:HelpGroup[]):HelpProgress{
 const defaults:HelpProgress={version:2,tab:'tour',articleId:'folders',steps:{}};
 try{
  const current=JSON.parse(localStorage.getItem(HELP_PROGRESS_KEY)??'null');
  const previous=current??JSON.parse(localStorage.getItem('smart-photo-indexer.help.v1')??'null');
  if(!previous||![1,2].includes(previous.version))return defaults;
  const result:HelpProgress={...defaults,tab:previous.tab==='reference'?'reference':previous.tab==='about'?'about':'tour',articleId:typeof previous.articleId==='string'?previous.articleId:'folders'};
  for(const context of ['main','editor','viewer'] as const){
   const saved=previous.steps?.[context];if(!saved)continue;
   const stepId=previous.version===1?(legacy as Record<string,string>)[saved]??saved:saved.stepId;
   const feature=features.find(f=>f.id===stepId&&f.context===context);
   if(!feature){result.stale=true;continue;}
   const scope=previous.version===2&&groups.some(group=>group.context===context&&group.id===saved.scope)&&feature.groupId===saved.scope?saved.scope:'all';
   result.steps[context]={stepId,scope};
  }
  return result;
 }catch{return defaults;}
}
export function writeHelpProgress(value:HelpProgress,context:HelpContext):boolean{
 try{
  let other:HelpProgress['steps']={};
  try{const saved=JSON.parse(localStorage.getItem(HELP_PROGRESS_KEY)??'null');if(saved?.version===2&&saved.steps&&typeof saved.steps==='object')other=saved.steps;}catch{/* Replace a damaged record, never prevent reading help. */}
  localStorage.setItem(HELP_PROGRESS_KEY,JSON.stringify({...value,stale:undefined,steps:{...value.steps,...other,[context]:value.steps[context]}}));return true;
 }catch{return false;}
}
