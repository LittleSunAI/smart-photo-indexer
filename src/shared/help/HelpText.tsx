import {text as localeText} from "../i18n/Runtime";
import type {ReactNode} from 'react';
import type {HelpMessage} from './HelpMessage';
import {richMessage,useCurrentLocale,type RichToken} from '../i18n/Runtime';

type Token=RichToken;
function content(tokens:Token[],args:Record<string,string|number>):ReactNode{
 return tokens.map((token,i)=>{
  if(typeof token==='string')return token;
  if(token.kind==='arg'){
   if(!Object.prototype.hasOwnProperty.call(args,token.name))throw Error(localeText("common", "ui.HelpText.3a4a643bfa499ef0", {"value1": (token.name)}));
   return String(args[token.name]);
  }
  const children=content(token.children,args);
  if(token.kind==='strong')return <strong key={i}>{children}</strong>;
  if(token.kind==='kbd')return <kbd key={i}>{children}</kbd>;
  return <code key={i}>{children}</code>;
 });
}
export function HelpText({message}:{message:HelpMessage}){
 useCurrentLocale();
 const blocks=richMessage(message.id);
 return <>{blocks.map((tokens,i)=><p key={i}>{content(tokens,message.args??{})}</p>)}</>;
}
