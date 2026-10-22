import {readFile,readdir,writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {join} from 'node:path';
import {createHash} from 'node:crypto';

const root=fileURLToPath(new URL('../',import.meta.url));
export const keyboard=/^(?:(?:Ctrl|Control|Shift|Alt|Win|Meta|Cmd)\s*\+\s*)*(?:Ctrl|Shift|Alt|Enter|Escape|Esc|Space|Leertaste|Espace|Tab|Delete|Backspace|Home|End|PageUp|PageDown|F\d{1,2}|[A-Z0-9]|[←→↑↓]|Пробел)$/i;
export function inline(text,slots=new Set(),format=true){
 const pattern=format?/\*\*([^*]+)\*\*|`([^`]+)`|\{([\p{L}][\p{L}\p{N}_-]*)\}/gu:/\{([\p{L}][\p{L}\p{N}_-]*)\}/gu;
 const tokens=[];let start=0;
 for(const match of text.matchAll(pattern)){
  if(match.index>start)tokens.push(text.slice(start,match.index));
  const name=format?match[3]:match[1];
  if(name){slots.add(name);tokens.push({kind:'arg',name});}
  else if(match[1])tokens.push({kind:'strong',children:inline(match[1],slots,false)});
  else tokens.push({kind:keyboard.test(match[2])?'kbd':'code',children:inline(match[2],slots,false)});
  start=match.index+match[0].length;
 }
 if(start<text.length)tokens.push(text.slice(start));
 return tokens;
}
const argumentPattern=/\{([\p{L}][\p{L}\p{N}_-]*)\}/gu;
export function simpleMessage(text){return text.replace(argumentPattern,(_,arg)=>`{{${arg}}}`);}
function strings(value,visit,path=[]){
 if(typeof value==='string')visit(value,path);
 else if(Array.isArray(value))value.forEach((v,i)=>strings(v,visit,[...path,i]));
 else if(value&&typeof value==='object')Object.entries(value).forEach(([k,v])=>strings(v,visit,[...path,k]));
 else throw Error(`Invalid author resource at ${path.join('.')}`);
}
function argumentNames(text){return [...text.matchAll(argumentPattern)].map(match=>match[1]).sort();}
export async function compileLocales({check=false}={}){
 const input=join(root,'src/shared/i18n/locales'),out=join(root,'src/shared/i18n/generated');
 const languages=['ru','en','de','fr'],bundles={},helpArguments={},sources=[],keys={};
 let baseline;
 for(const language of languages){
  const bundle={},author={};
  for(const name of (await readdir(join(input,language))).filter(n=>n.endsWith('.json')).sort()){
   const raw=(await readFile(join(input,language,name),'utf8')).replace(/\r\n?/g,'\n'),ns=name.slice(0,-5),data=JSON.parse(raw);
   if(!data||Array.isArray(data)||typeof data!=='object')throw Error(`Invalid namespace ${language}/${ns}`);
   author[ns]=data;sources.push({file:`${language}/${name}`,sha256:createHash('sha256').update(raw).digest('hex')});
  }
  if(language==='ru')baseline=author;
  else for(const [ns,data] of Object.entries(author)){
   if(JSON.stringify(Object.keys(author).sort())!==JSON.stringify(Object.keys(baseline).sort()))throw Error(`Mismatched namespaces ${language}`);
   if(ns==='plurals')continue;
   if(JSON.stringify(Object.keys(data).sort())!==JSON.stringify(Object.keys(baseline[ns]).sort()))throw Error(`Mismatched keys ${language}/${ns}`);
   for(const [key,value] of Object.entries(data))if(typeof value==='string'){
    if(!value.trim()||JSON.stringify(argumentNames(value))!==JSON.stringify(argumentNames(baseline[ns][key])))throw Error(`Mismatched message arguments ${language}/${ns}/${key}`);
   }
  }
  const rich={},args={};
  const compileRich=(id,paragraphs)=>{
   if(!Array.isArray(paragraphs)||!paragraphs.length||paragraphs.some(p=>typeof p!=='string'||!p.trim()))throw Error(`Invalid rich text ${language}/${id}`);
   const slots=new Set();rich[id]=paragraphs.map(p=>inline(p,slots));args[id]=[...slots].sort();
   if(language==='ru')helpArguments[id]=args[id];
   else if(JSON.stringify(args[id])!==JSON.stringify(helpArguments[id]))throw Error(`Mismatched rich arguments ${language}/${id}`);
  };
  for(const [id,paragraphs] of Object.entries(author.help))compileRich(id,paragraphs);
  for(const [ns,data] of Object.entries(author)){
   if(ns==='help')continue;
   bundle[ns]={};
   for(const [key,value] of Object.entries(data)){
    if(ns==='tour'&&value&&typeof value==='object'&&!Array.isArray(value)){
     if(typeof value.title!=='string')throw Error(`Missing tour title ${language}/${key}`);
     bundle[ns][key+'.title']=value.title;
     if(value.bodyRef){
      if(value.paragraphs||!rich[value.bodyRef])throw Error(`Invalid shared body ${language}/${key}`);
     }else if(value.paragraphs)compileRich('tour.'+key,value.paragraphs);
     continue;
    }
    if(typeof value==='string')bundle[ns][key]=simpleMessage(value);
    else{strings(value,()=>{});bundle[ns][key]=value;}
   }
   if(language==='ru')keys[ns]=Object.keys(bundle[ns]);
  }
  bundle.help=rich;bundles[language]=bundle;
  const pluralFamilies=data=>[...new Set(Object.keys(data).map(key=>key.replace(/_(one|few|many|other)$/,'')))].sort();
  const families=pluralFamilies(author.plurals);
  if(JSON.stringify(families)!==JSON.stringify(pluralFamilies(baseline.plurals)))throw Error('Mismatched plural families '+language);
  for(const family of families){
   const other=author.plurals[family+'_other'];if(typeof other!=='string')throw Error('Missing plural other '+language+'/'+family);
   for(const [key,value] of Object.entries(author.plurals).filter(([key])=>key.startsWith(family+'_'))){if(JSON.stringify(argumentNames(value))!==JSON.stringify(argumentNames(baseline.plurals[family+'_other'])))throw Error('Mismatched plural args '+language+'/'+key);}
   // CLDR may add categories (for example French million). Stay in the selected language.
   for(const category of new Intl.PluralRules(language).resolvedOptions().pluralCategories)bundle.plurals[family+'_'+category]??=simpleMessage(other);
  }
 }
 const parameters=JSON.parse(await readFile(join(root,'docs/ui-help/MODEL_PARAMETER_MAP.json'),'utf8'));
 const overrides=JSON.parse(await readFile(join(root,'docs/ui-help/MODEL_PARAMETER_ROUTE_OVERRIDES.json'),'utf8'));
 for(const row of [...parameters.entries,...overrides.entries])if(!helpArguments[row.helpId]||helpArguments[row.helpId].length)throw Error(`Invalid parameter help: ${row.helpId}`);
 const header='// Generated by scripts/compile-locales.mjs. Edit src/shared/i18n/locales instead.\n';
 const types=header+'export interface HelpArguments {\n'+Object.entries(helpArguments).map(([id,args])=>`  ${JSON.stringify(id)}: ${args.length?'{'+args.map(arg=>`${JSON.stringify(arg)}: string | number`).join('; ')+'}':'never'};`).join('\n')+'\n}\n'+
 'export type HelpId = keyof HelpArguments;\nexport type HelpMessage = {[K in HelpId]: {id: K} & (HelpArguments[K] extends never ? {args?: never} : {args: HelpArguments[K]})}[HelpId];\n';
 const parameterData={keys:Object.fromEntries(parameters.entries.map(r=>[r.schemaKey,r.helpId])),routes:Object.fromEntries(overrides.entries.map(r=>[r.routeId+'|'+r.schemaKey,r.helpId]))};
 const tourMessageIds=Object.fromEntries(Object.entries(baseline.tour).filter(([,value])=>value&&typeof value==='object'&&(value.bodyRef||value.paragraphs)).map(([id,value])=>[id,value.bodyRef??'tour.'+id]));
 const products=languages.map(language=>[join(out,language+'.json'),JSON.stringify(bundles[language])+'\n']);
 const nativeBindings=JSON.parse(await readFile(join(root,'src/shared/i18n/native-caption-bindings.json'),'utf8'));
 const arms=[];
 for(const language of languages)for(const key of nativeBindings.keys){const value=bundles[language].native[key];if(typeof value!=='string'||/[{}]/.test(value))throw Error('Invalid native caption '+language+'/'+key);arms.push(`  (${JSON.stringify(language)},${JSON.stringify(key)})=>${JSON.stringify(value)},`);}
 const nativeSource=header+'pub fn caption(locale:Option<&str>,key:&str)->&\'static str {\n let language=match locale {Some("en")=>"en",Some("de")=>"de",Some("fr")=>"fr",_=>"ru"};\n match (language,key) {\n'+arms.join('\n')+'\n  _=>"",\n }\n}\n';
 const nsisEscape=value=>value.replaceAll('$','$$').replaceAll('"','$\\"').replaceAll('\n','$\\r$\\n');
 const nsis='; Generated from src/shared/i18n/locales. Do not edit.\n'+languages.map(language=>['shell.image-type','shell.application-description'].map((key,index)=>`LangString ${index?'SmartPhotoApplicationDescription':'SmartPhotoImageType'} ${{ru:1049,en:1033,de:1031,fr:1036}[language]} "${nsisEscape(bundles[language].native[key])}"`).join('\n')).join('\n')+'\n';
 products.push([join(root,'src-tauri/src/ui_locale_generated.rs'),nativeSource],[join(root,'src-tauri/windows/locale-strings.nsh'),nsis]);
 products.push([join(root,'src/shared/help/HelpMessage.ts'),types],
  [join(root,'src/shared/help/help-catalog.json'),JSON.stringify(bundles.ru.help)+'\n'],
  [join(root,'src/shared/help/help-catalog-sources.json'),JSON.stringify(sources,null,2)+'\n'],
  [join(root,'src/shared/help/parameter-help.json'),JSON.stringify(parameterData,null,2)+'\n'],
  [join(out,'tour-message-ids.json'),JSON.stringify(tourMessageIds,null,2)+'\n'],
  [join(out,'ResourceKeys.ts'),header+'export interface ResourceKeys {\n'+Object.entries(keys).map(([ns,ids])=>` ${JSON.stringify(ns)}: ${ids.map(id=>JSON.stringify(id)).join(' | ')};`).join('\n')+'\n}\n']);
 if(!check)await mkdir(out,{recursive:true});
 for(const [path,content] of products){if(check){if((await readFile(path,'utf8')).replace(/\r\n?/g,'\n')!==content)throw Error(`Stale generated locale: ${path}`);}else await writeFile(path,content);}
 return {status:check?'locale-resources-current':'locale-resources-compiled',languages,helpEntries:Object.keys(helpArguments).length,sourceFiles:sources.length};
}
