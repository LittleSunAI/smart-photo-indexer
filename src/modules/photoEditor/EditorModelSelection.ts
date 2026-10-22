import type {EditorDraft} from './EditorContracts';
import {choiceFor,type ModelChoice} from './EditorModelCatalog';
import {commonFormValues} from './EditorRequestIdentity';
import {supportsCustomSize} from './EditorOutputSize';
type SelectionRoute={id:string;schemaSha256:string;inputSchema:{properties:Record<string,unknown>};definitions?:Record<string,unknown>};
const reserved=['sync_mode','enable_safety_checker','auto_crop'];
export function modelContextKey(operation:string,choice:ModelChoice,routes:SelectionRoute[]):string{return `${operation}|${choice.id}|${routes.find(r=>r.id===choice.route)?.schemaSha256??'unverified'}`;}
export function modelContextValues(values:EditorDraft['formValues']):EditorDraft['formValues']{
 return Object.fromEntries(Object.entries(values).filter(([key])=>!key.startsWith('__')||['__parameterModes','__outputSizeMode','__rawJSON','__rawJSONError','__jsonParameterBase','__jsonOutputSizeBase','__invalidFields'].includes(key)));
}
/** The filter price preview and the actual selection must use the same parameters. */
export function selectModelDraft(d:EditorDraft,choice:ModelChoice,routes:SelectionRoute[]):EditorDraft{
 const key=(c:ModelChoice)=>modelContextKey(d.operation,c,routes);
 const contexts={...(d.formValues.__contexts as Record<string,{formValues?:Record<string,unknown>}>??{})},previous=choiceFor(d);
 if(previous)contexts[key(previous)]={formValues:modelContextValues(d.formValues)};
 const route=routes.find(r=>r.id===choice.route);
 const legacy=contexts[`${d.operation}|${choice.route}|${route?.schemaSha256??'unverified'}`]?.formValues;
 const saved=modelContextValues(contexts[key(choice)]?.formValues??(legacy?.__choiceId===choice.id?legacy:{}));
 const canSource=!['upscale','outpaint'].includes(d.operation)&&supportsCustomSize(route?.inputSchema.properties.image_size as Parameters<typeof supportsCustomSize>[0],route?.definitions as Parameters<typeof supportsCustomSize>[1]);
 const sizeMode=canSource?(saved.__outputSizeMode??(Object.hasOwn(saved,'image_size')?'model':'source')):'model';
 return {...d,profile:choice.tier,modelId:choice.route,formValues:{...Object.fromEntries(Object.entries(choice.preset).filter(([k])=>!reserved.includes(k))),...commonFormValues(d.formValues),...saved,__outputSizeMode:sizeMode,__choiceId:choice.id,__schemaSHA:route?.schemaSha256,__contexts:contexts,__lastChoices:{...(d.formValues.__lastChoices as Record<string,string>??{}),[`${d.operation}|${choice.tier}`]:choice.id,[d.operation]:choice.id},__customProfile:Object.entries(saved).some(([k,v])=>JSON.stringify(v)!==JSON.stringify(choice.preset[k]))}};
}
