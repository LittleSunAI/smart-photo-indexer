import {text as localeText, useCurrentLocale} from "../i18n/Runtime";
import {useHint} from '../help/useHint';
import type {HelpMessage} from '../help/HelpMessage';
export function InfoHint({label,text,message,activationOnly=false}:{label:string;text?:string;message?:HelpMessage;activationOnly?:boolean}){
 const ref=useHint<HTMLButtonElement>({content:text,message,explicit:true,activationOnly});
 return <span className="info-hint"><button ref={ref} type="button" aria-label={label} aria-expanded={false}>?</button></span>;
}
export function HintTitle({title,text,label,message}:{title:string;text?:string;label?:string;message?:HelpMessage}){
  useCurrentLocale();
 return <span className="section-title"><span className="section-title__text">{title}</span><InfoHint label={label??localeText("common", "ui.InfoHint.170afba58fe68047", {"value1": (title)})} text={text} message={message}/></span>;
}
