import catalog from './model-catalog.json';
import type {AnalysisOutputMode} from '../../modules/settings/Settings.types';

export function analysisLimits(provider:string,model:string,outputMode:AnalysisOutputMode,maxTokens:number,imageMaxSide:number){
  const row=catalog.local.find(row=>row.id===model);
  const fast=provider==='local'&&Boolean(row&&'fastLocal' in row&&row.fastLocal);
  const tagsOnly=outputMode==='tags';
  return {maxTokens:tagsOnly?Math.min(640,maxTokens):fast?Math.min(1024,maxTokens):maxTokens,imageMaxSide:tagsOnly?Math.min(768,imageMaxSide):fast?Math.min(1024,imageMaxSide):imageMaxSide,fast};
}
