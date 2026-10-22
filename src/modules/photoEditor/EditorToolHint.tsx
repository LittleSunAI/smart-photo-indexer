import type {ReactElement} from 'react';
import {useHint} from '../../shared/help/useHint';
import type {HelpMessage} from '../../shared/help/HelpMessage';
import './editor-hint.css';

/** Keeps the existing display:contents adapter; registers its actual control. */
export function EditorToolHint({text,message,children}:{text?:string;message?:HelpMessage;children:ReactElement}){
 const ref=useHint<HTMLSpanElement>({content:text,message,target:root=>root.querySelector<HTMLElement>('button,input,select,[tabindex]')});
 return <span ref={ref} className="editor-tool-hint">{children}</span>;
}
