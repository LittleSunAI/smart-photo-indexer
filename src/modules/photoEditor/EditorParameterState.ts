import {text as localeText} from "../../shared/i18n/Runtime";
import type {EditorDraft} from './EditorContracts';

export type ParameterMode='preset'|'provider-default'|'explicit';
export type ParameterModes=Record<string,ParameterMode>;
export function parameterMode(d:EditorDraft,key:string):ParameterMode {
  return (d.formValues.__parameterModes as ParameterModes|undefined)?.[key]??'preset';
}
export function setParameter(d:EditorDraft,key:string,value:unknown,mode:ParameterMode=value===undefined?'provider-default':'explicit'):EditorDraft {
  const values={...d.formValues},modes={...(values.__parameterModes as ParameterModes|undefined),[key]:mode};
  if(mode==='provider-default'||value===undefined)delete values[key];else values[key]=value;
  // A structured edit wins over an older JSON override of the same field.
  const base={...(values.__jsonParameterBase as Record<string,unknown>|undefined)};
  if(Object.hasOwn(base,key)){
    delete base[key];values.__jsonParameterBase=base;
    try{const parsed=JSON.parse(String(values.__rawJSON??'{}'));if(parsed&&typeof parsed==='object'&&!Array.isArray(parsed)){delete parsed[key];values.__rawJSON=Object.keys(parsed).length?JSON.stringify(parsed,null,2):'';}}catch{/* The invalid JSON remains visible until corrected. */}
    if(!Object.hasOwn(base,'image_size')&&!Object.hasOwn(base,'resolution'))delete values.__jsonOutputSizeBase;
  }
  const invalid={...(values.__invalidFields as Record<string,string>|undefined)};delete invalid[key];
  return {...d,formValues:{...values,__parameterModes:modes,__invalidFields:invalid,__customProfile:true}};
}

/** JSON owns only its keys. Removing a JSON key restores its previous structured value/mode. */
export function updateParameterJSON(d:EditorDraft,raw:string):EditorDraft {
  try {
    const parsed:unknown=raw.trim()?JSON.parse(raw):{};
    if(!parsed||Array.isArray(parsed)||typeof parsed!=='object'||Object.keys(parsed).some(k=>k.startsWith('__')))throw Error(localeText("editor", "ui.EditorParameterState.fb6b8a4dabaa1ab3"));
    const values={...d.formValues},modes={...(values.__parameterModes as ParameterModes|undefined)};
    const previous=(values.__jsonParameterBase as Record<string,{present:boolean;value:unknown;mode:ParameterMode}>|undefined)??{};
    const base={...previous};
    for(const [key,saved] of Object.entries(previous)) {
      if(saved.present)values[key]=saved.value;else delete values[key];
      modes[key]=saved.mode;
      if(!(key in parsed))delete base[key];
    }
    for(const [key,value] of Object.entries(parsed)) {
      if(!base[key])base[key]={present:Object.hasOwn(values,key),value:values[key],mode:modes[key]??'preset'};
      values[key]=value;modes[key]='explicit';
    }
    const hasSize=Object.hasOwn(parsed,'image_size')||Object.hasOwn(parsed,'resolution');
    if(hasSize){
      if(!values.__jsonOutputSizeBase)values.__jsonOutputSizeBase={present:Object.hasOwn(values,'__outputSizeMode'),value:values.__outputSizeMode};
      values.__outputSizeMode='model';
    }else if(values.__jsonOutputSizeBase){
      const saved=values.__jsonOutputSizeBase as {present:boolean;value:unknown};
      if(saved.present)values.__outputSizeMode=saved.value;else delete values.__outputSizeMode;
      delete values.__jsonOutputSizeBase;
    }
    return {...d,formValues:{...values,__parameterModes:modes,__jsonParameterBase:base,__rawJSON:raw,__rawJSONError:false,__invalidFields:{},__customProfile:true}};
  } catch {
    return {...d,formValues:{...d.formValues,__rawJSON:raw,__rawJSONError:true}};
  }
}
