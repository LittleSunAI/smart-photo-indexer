import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {setParameter} from './EditorParameterState';
import {useEffect,useRef,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {choicesFor,tierName,type ModelChoice} from './EditorModelCatalog';
import {selectModelDraft} from './EditorModelSelection';
import {priceIdentity} from './EditorRequestIdentity';
import {supportsCustomSize} from './EditorOutputSize';
import type {EditorDraft} from './EditorContracts';
type Route=Parameters<typeof selectModelDraft>[2][number];
export interface ModelFilter {tier:''|EditorDraft['profile'];maxPrice:string;source:boolean;size:''|'1K'|'2K'|'3K'|'4K'|'custom';width:string;height:string;references:string}
export const emptyModelFilter=():ModelFilter=>({tier:'',maxPrice:'',source:false,size:'',width:'',height:'',references:''});
export interface ModelMatch {id:string;priceMicros:number|null;sourceSupported:boolean;sizeSupported:boolean;maxReferences:number;reason?:string}
export function applyModelSizeFilter(d:EditorDraft,filter:ModelFilter,route?:Route,dimensions?:{width:number;height:number}):EditorDraft|null{
 if(filter.source)return {...d,formValues:{...d.formValues,__outputSizeMode:'source'}};
 if(!filter.size)return d;
 if(['upscale','outpaint'].includes(d.operation)||!route)return null;
 const properties=route.inputSchema.properties as Record<string,{enum?:string[];anyOf?:Array<{enum?:string[]}>}>;
 const resolution=properties.resolution?.enum??properties.resolution?.anyOf?.flatMap(s=>s.enum??[]);
 const resolutionValue=resolution?.find(value=>value.toUpperCase()===filter.size);
 if(filter.size!=='custom'&&resolutionValue){const next=setParameter(d,'resolution',resolutionValue);return {...next,formValues:{...next.formValues,__outputSizeMode:'model'}};}
 if(!properties.image_size)return null;
 const presets=properties.image_size.enum??properties.image_size.anyOf?.flatMap(s=>s.enum??[]);
 if(filter.size!=='custom'&&presets?.includes('auto_'+filter.size))return {...setParameter(d,'image_size','auto_'+filter.size),formValues:{...setParameter(d,'image_size','auto_'+filter.size).formValues,__outputSizeMode:'model'}};
 if(!supportsCustomSize(properties.image_size,route.definitions as Parameters<typeof supportsCustomSize>[1]))return null;
 let width:number,height:number;
 if(filter.size==='custom'){width=Number(filter.width);height=Number(filter.height);if(!Number.isSafeInteger(width)||!Number.isSafeInteger(height)||width<1||height<1)return null;}
 else{const edge=Number(filter.size[0])*1024,source=dimensions??{width:1024,height:1024},scale=edge/Math.max(source.width,source.height);width=Math.max(1,Math.round(source.width*scale/32)*32);height=Math.max(1,Math.round(source.height*scale/32)*32);}
 const next=setParameter(d,'image_size',{width,height});return {...next,formValues:{...next.formValues,__outputSizeMode:'model'}};
}
export function filterModels(choices:ModelChoice[],filter:ModelFilter,matches:ModelMatch[]):ModelChoice[]{
 const price=filter.maxPrice.trim()===''?null:Number(filter.maxPrice);
 const refs=filter.references.trim()===''?0:Number(filter.references);
 return choices.filter(c=>{const m=matches.find(m=>m.id===c.id);return (!filter.tier||c.tier===filter.tier)&&(!filter.source||m?.sourceSupported===true)&&(!filter.size||m?.sizeSupported===true)&&Number.isInteger(refs)&&refs>=0&&(refs===0||m!==undefined&&m.maxReferences>=refs)&&(price===null||Number.isFinite(price)&&price>=0&&m?.priceMicros!==null&&m?.priceMicros!==undefined&&m.priceMicros<=price*1e6);});
}
export function EditorModelFilters({draft,epoch,routes,filter,onFilter,onMatches,source}:{draft:EditorDraft;epoch:string;routes:Array<Route&{maxInputs:number;pricePolicy?:string}>;filter:ModelFilter;onFilter(filter:ModelFilter):void;onMatches(matches:ModelMatch[],loading:boolean):void;source?:{width:number;height:number}}){
  useCurrentLocale();
 const [loading,setLoading]=useState(false),[error,setError]=useState('');const latest=useRef({draft,routes,onMatches});latest.current={draft,routes,onMatches};
 // Point movement does not change tariff factors or size capabilities.
 const coordinateTariff=choicesFor(draft.operation).some(choice=>routes.find(route=>route.id===choice.route)?.pricePolicy==='token');
 const identity=JSON.stringify({...JSON.parse(priceIdentity(draft,coordinateTariff)),annotations:draft.annotations.map(a=>({id:a.id,kind:a.kind,included:a.included})),contexts:draft.formValues.__contexts,pricing:draft.formValues.__pricingRevision}),routeKey=routes.map(r=>r.schemaSha256).join('|');
 useEffect(()=>{let disposed=false;const timer=setTimeout(()=>void(async()=>{setLoading(true);setError('');latest.current.onMatches([],true);const {draft,routes}=latest.current;const matches:ModelMatch[]=[];
  try{for(const choice of choicesFor(draft.operation)){
   const r=routes.find(r=>r.id===choice.route),base=selectModelDraft(draft,choice,routes),candidate=applyModelSizeFilter(base,filter,r,source);let priceMicros:number|null=null,sourceSupported=false,sizeSupported=false,reason:string|undefined;
   if(candidate)try{const p=await invoke<{reservedMicros:number}>('probe_editor_model',{epoch,draft:candidate});priceMicros=p.reservedMicros;sizeSupported=true;}catch(e){reason=String(e);}
   if(disposed)return;
   if(!['upscale','outpaint'].includes(draft.operation))try{await invoke('probe_editor_model',{epoch,draft:{...base,formValues:{...base.formValues,__outputSizeMode:'source'}}});sourceSupported=true;}catch{}
   if(disposed)return;matches.push({id:choice.id,priceMicros,sourceSupported,sizeSupported,maxReferences:Math.max(0,(r?.maxInputs??1)-1),reason});
  }latest.current.onMatches(matches,false);
  }catch{if(!disposed){setError(localeText("editor", "ui.EditorModelFilters.0a796da5a4a2e1de"));latest.current.onMatches([],false);}}
  finally{if(!disposed)setLoading(false);}
 })(),220);return()=>{disposed=true;clearTimeout(timer);};},[identity,epoch,routeKey,filter.source,filter.size,filter.width,filter.height,source?.width,source?.height]);
 return <div className="editor-model-filters" aria-label={localeText("editor", "ui.EditorModelFilters.ba18010b7173f30c")}>
  <Hint message={{id:"model.filter-tier"}}><label>{localeText("editor", "ui.EditorModelFilters.f89b11d89cdeb2ec")}<select aria-label={localeText("editor", "ui.EditorModelFilters.bc34092ba4279e2b")} value={filter.tier} onChange={e=>onFilter({...filter,tier:e.target.value as ModelFilter['tier']})}><option value="">{localeText("editor", "ui.EditorModelFilters.851b047cbf6a1d6f")}</option>{(['economy','balanced','quality'] as const).map(t=><option key={t} value={t}>{tierName(t)}</option>)}</select></label></Hint>
  <Hint message={{id:"model.filter-price"}}><label>{localeText("editor", "ui.EditorModelFilters.c6066a86ac23c4d5")}<input type="number" aria-label={localeText("editor", "ui.EditorModelFilters.9953f75a012f8028")} min={0} step={0.01} placeholder={localeText("editor", "ui.EditorModelFilters.15df81f60ac5d491")} value={filter.maxPrice} onChange={e=>onFilter({...filter,maxPrice:e.target.value})}/></label></Hint>
  <Hint message={{id:"model.filter-resolution"}}><label>{localeText("editor", "ui.EditorModelFilters.86eff9db140aaebe")}<select aria-label={localeText("editor", "ui.EditorModelFilters.5a381e38dba8987f")} value={filter.size} disabled={['upscale','outpaint'].includes(draft.operation)} onChange={e=>onFilter({...filter,size:e.target.value as ModelFilter['size'],source:false})}><option value="">{localeText("editor", "ui.EditorModelFilters.bfef6afe99eb8378")}</option>{['1K','2K','3K','4K'].map(v=><option key={v} value={v}>{v}</option>)}<option value="custom">{localeText("editor", "ui.EditorModelFilters.4553a58a968e5693")}</option></select></label></Hint>
  <Hint message={{id:"model.filter-references"}}><label title={localeText("editor", "ui.EditorModelFilters.1752feff84a59c47")}>{localeText("editor", "ui.EditorModelFilters.6cdfb627959b4408")}<input type="number" aria-label={localeText("editor", "ui.EditorModelFilters.989feb29a6d9de1e")} min={0} max={15} step={1} placeholder={localeText("editor", "ui.EditorModelFilters.e2aabe69a4c6b69e")} value={filter.references} onChange={e=>onFilter({...filter,references:e.target.value})}/></label></Hint>
  {filter.size==='custom'?<><Hint message={{id:"model.filter-width"}}><label>{localeText("editor", "ui.EditorModelFilters.2231094b1e18e730")}<input aria-label={localeText("editor", "ui.EditorModelFilters.559deb880850887a")} type="number" min={1} max={14142} step={1} value={filter.width} onChange={e=>onFilter({...filter,width:e.target.value})}/></label></Hint><Hint message={{id:"model.filter-height"}}><label>{localeText("editor", "ui.EditorModelFilters.0eec74a0fe5920a1")}<input aria-label={localeText("editor", "ui.EditorModelFilters.ab164785a7aa75bc")} type="number" min={1} max={14142} step={1} value={filter.height} onChange={e=>onFilter({...filter,height:e.target.value})}/></label></Hint></>:null}
  <Hint message={{id:"model.filter-source-size"}}><label className="editor-filter-source" title={localeText("editor", "ui.EditorModelFilters.3d7bcfada31195c9")}><input type="checkbox" aria-label={localeText("editor", "ui.EditorModelFilters.44f7cbb5c90198d7")} checked={filter.source} disabled={['upscale','outpaint'].includes(draft.operation)} onChange={e=>onFilter({...filter,source:e.target.checked,size:''})}/>{localeText("editor", "ui.EditorModelFilters.8799cfca6e56fb64")}</label></Hint>
  <span className="editor-filter-note" role="status">{error|| (loading?localeText("editor", "ui.EditorModelFilters.90a50a64c5518b51"):localeText("editor", "ui.EditorModelFilters.ad4fe569441d0b26"))}</span>
 </div>;
}
