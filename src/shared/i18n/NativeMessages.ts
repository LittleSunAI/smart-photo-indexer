import {i18n} from './Runtime';
import originals from './native-display-bindings.json';
const exact=new Map<string,string>(),templates:Array<{key:string;pattern:RegExp;slots:string[]}>=[];
const escape=(value:string)=>value.replace(/[.*+?^\$\{\}()|[\]\\]/g,'\\$&');
for(const [key,original] of Object.entries(originals)){
 const slots:string[]=[],pieces:string[]=[];let start=0;
 for(const match of original.matchAll(/(?<!\{)\{([^{}]*)\}(?!\})/g)){
  pieces.push(escape(original.slice(start,match.index)),'([\\s\\S]*?)');slots.push(match[1].split(':')[0]);start=match.index!+match[0].length;
 }
 if(!slots.length)exact.set(original,key);
 else if(original.replace(/\{[^{}]*\}/g,'').length>=8){pieces.push(escape(original.slice(start)));templates.push({key,slots,pattern:new RegExp('^'+pieces.join('')+'$')});}
}
function resource(key:string){const locale=i18n.resolvedLanguage??'ru';return i18n.getResource(locale,'native',key)??i18n.getResource('ru','native',key);}
/** Presentation of old native diagnostics only; never use the result as a status or API value. */
function originalDisplayText(original:string):string{
 const key=exact.get(original);if(key){const value=resource(key);return typeof value==='string'?value:original;}
 for(const rule of templates){const match=rule.pattern.exec(original);if(!match)continue;const translated=resource(rule.key);if(typeof translated!=='string')return original;
  const named=Object.fromEntries(rule.slots.map((slot,index)=>[slot,match[index+1]]));let ordinal=0;
  return translated.replace(/\{\{([^{}]+)\}\}|\{([^{}]*)\}/g,(token,namedSlot,rustSlot)=>{const slot=(namedSlot??rustSlot).split(':')[0];return slot?named[slot]??token:match[++ordinal]??token;});
 }
 return original;
}

// Resolve already-rendered diagnostics back to their stable resource before translating.
// Receipts and technical payloads retain their original strings; only presentation changes.
let aliasLocale='',aliases=new Map<string,string>();
function currentAliases(){
 const locale=i18n.resolvedLanguage??'ru';if(aliasLocale===locale)return aliases;
 aliasLocale=locale;aliases=new Map();
 for(const lang of ['ru','en','de','fr']){const bundle=i18n.getResourceBundle(lang,'native');if(!bundle)continue;for(const[key,value]of Object.entries(bundle)){if(typeof value==='string'&&!/[{}]/.test(value)&&key in originals)aliases.set(value,key);}}
 return aliases;
}
export function nativeDisplayText(original:string):string{
 const raw=original.replace(/^(?:Error|NativeCommandFailure):\s*/,''),key=currentAliases().get(raw);
 if(key){const translated=resource(key);if(typeof translated==='string')return translated;}
 const translated=originalDisplayText(raw);return translated===raw?original:translated;
}
/** Static UI feedback stored by an async controller remains localizable without repeating it. */
export function uiFeedbackText(ns:string,original:string):string{
 const direct=nativeDisplayText(original);if(direct!==original)return direct;
 for(const lang of ['ru','en','de','fr']){const bundle=i18n.getResourceBundle(lang,ns);if(!bundle)continue;for(const[key,value]of Object.entries(bundle)){if(value===original){const current=i18n.getResource(i18n.resolvedLanguage??'ru',ns,key);if(typeof current==='string')return current;}}}
 return original;
}
