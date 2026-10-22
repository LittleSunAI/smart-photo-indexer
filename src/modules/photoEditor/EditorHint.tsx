import type {ReactNode} from 'react';
import {useHint} from '../../shared/help/useHint';
import type {HelpMessage} from '../../shared/help/HelpMessage';
import './editor-hint.css';
export function EditorHint({label,children,message,supplement}:{label:string;children?:ReactNode;message?:HelpMessage;supplement?:HelpMessage}){
 const ref=useHint<HTMLButtonElement>({content:children,message,supplement,details:message?.id==='model.help'?children:undefined,explicit:true});
 return <span className="editor-context-help"><button ref={ref} type="button" aria-label={label} aria-expanded={false}>?</button></span>;
}
