// URLs only, never decoded pixel buffers. Native validation still runs on mount.
const urls=new Map<string,{url:string;at:number}>();
const limit=256,ttl=60_000;
export function previewKey(path:string,width:number,height:number,revision?:string|number){return JSON.stringify([path,width,height,revision??null]);}
export function rememberedPreview(key:string):string|null{
  const entry=urls.get(key);if(!entry)return null;
  if(Date.now()-entry.at>ttl){urls.delete(key);return null;}
  urls.delete(key);urls.set(key,entry);return entry.url;
}
export function rememberPreview(key:string,url:string){urls.delete(key);urls.set(key,{url,at:Date.now()});while(urls.size>limit){urls.delete(urls.keys().next().value!);}}
export function forgetPreview(key:string){urls.delete(key);}
export function forgetAllPreviews(){urls.clear();}
