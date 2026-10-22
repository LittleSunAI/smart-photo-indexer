import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {Icon} from '../../shared/ui/Icon';
import {descriptionText,readDescriptionDocument,renderDescriptionDocument,safeDescriptionLink,type DescriptionFormat} from './DescriptionFormat';

export function DescriptionEditor({text,format,disabled,onChange}:{text:string;format?:DescriptionFormat|null;disabled:boolean;onChange(text:string,format:DescriptionFormat):void}){
  useCurrentLocale();
 const editor=useRef<HTMLDivElement>(null),range=useRef<Range|null>(null);
 const [expanded,setExpanded]=useState(false),[linkOpen,setLinkOpen]=useState(false),[link,setLink]=useState(''),[error,setError]=useState('');
 const [empty,setEmpty]=useState(!text.trim());
 useEffect(()=>{
  const node=editor.current;if(!node)return;
  const actual=readDescriptionDocument(node);
  if(descriptionText(actual)!==text.trim()||format&&JSON.stringify(actual)!==JSON.stringify(format))renderDescriptionDocument(node,text,format);
  setEmpty(!descriptionText(readDescriptionDocument(node)).trim());
 },[text,JSON.stringify(format)]);
 const remember=()=>{const selection=window.getSelection();if(selection?.rangeCount&&editor.current?.contains(selection.anchorNode))range.current=selection.getRangeAt(0).cloneRange();};
 const publish=()=>{if(editor.current){const document=readDescriptionDocument(editor.current),value=descriptionText(document);setEmpty(!value.trim());onChange(value,document);remember();}};
 const command=(name:string,value?:string)=>{
  const node=editor.current;if(!node||disabled)return;node.focus();
  if(range.current&&node.contains(range.current.commonAncestorContainer)){const selection=window.getSelection();selection?.removeAllRanges();selection?.addRange(range.current);}
  // WebView2 keeps the native edit history. Support is checked on the shipped runtime.
  if(!document.queryCommandSupported(name)){setError(localeText("details", "ui.DescriptionEditor.ee8405ba7682cd5c"));return;}
  document.execCommand(name,false,value);setError('');publish();
 };
 const commands=[['bold',localeText("details", "ui.DescriptionEditor.366576fb74ad3878"),'B'],['italic',localeText("details", "ui.DescriptionEditor.0f4893a335dbb6bb"),'I'],['underline',localeText("details", "ui.DescriptionEditor.0c4fe9e51a3a488e"),'U'],['insertUnorderedList',localeText("details", "ui.DescriptionEditor.d3d4c759f0fd2cfd"),'•'],['insertOrderedList',localeText("details", "ui.DescriptionEditor.b648b639239e1796"),'1.'],['undo',localeText("details", "ui.DescriptionEditor.1b61d3605d4ed731"),'↶'],['redo',localeText("details", "ui.DescriptionEditor.a695f8e1aa3d2186"),'↷']] as const;
 return <div data-tour-id="main.description.edit" className={`description-editor${expanded?' description-editor--expanded':''}`} onKeyDown={event=>{
  if(event.key==='Escape'&&(linkOpen||expanded)){event.preventDefault();event.stopPropagation();if(linkOpen)setLinkOpen(false);else setExpanded(false);editor.current?.focus();}
  if((event.ctrlKey||event.metaKey)&&['b','i','u'].includes(event.key.toLowerCase())){event.preventDefault();command(({b:'bold',i:'italic',u:'underline'} as Record<string,string>)[event.key.toLowerCase()]);}
  if(expanded&&event.key==='Tab'){const nodes=[...event.currentTarget.querySelectorAll<HTMLElement>('button:not(:disabled),input,[contenteditable=true]')];const index=nodes.indexOf(document.activeElement as HTMLElement);if(event.shiftKey&&index===0||!event.shiftKey&&index===nodes.length-1){event.preventDefault();nodes[event.shiftKey?nodes.length-1:0]?.focus();}}
 }}>
  <div className="description-editor__toolbar" role="toolbar" aria-label={localeText("details", "ui.DescriptionEditor.dfc287c68c4ded3e")}>
    {commands.map(([name,title,label])=><Hint key={name} message={{id:({bold:'text.bold',italic:'text.italic',underline:'text.underline',insertUnorderedList:'text.bullets',insertOrderedList:'text.numbered',undo:'text.undo',redo:'text.redo'} as const)[name]}}><button key={name} type="button" aria-label={title} title={title} disabled={disabled} onMouseDown={event=>event.preventDefault()} onClick={()=>command(name)} className={`description-editor__${name}`}>{name==='undo'||name==='redo'?<Icon name={name}/>:name==='insertUnorderedList'?<Icon name="bulletList"/>:name==='insertOrderedList'?<Icon name="numberedList"/>:label}</button></Hint>)}
    <Hint message={{id:"text.link"}}><button type="button" aria-label={localeText("details", "ui.DescriptionEditor.f305459fc274a33c")} title={localeText("details", "ui.DescriptionEditor.accce740bcdc5d80")} disabled={disabled} onMouseDown={event=>event.preventDefault()} onClick={()=>{remember();setLinkOpen(value=>!value);setLink('');}}><Icon name="link"/></button></Hint>
   <Hint message={{id:expanded?'text.collapse':'text.expand'}}><button type="button" aria-label={expanded?localeText("details", "ui.DescriptionEditor.bbe53be02568476e"):localeText("details", "ui.DescriptionEditor.6bdc05b21c759fa8")} title={expanded?localeText("details", "ui.DescriptionEditor.878e6c79f691debe"):localeText("details", "ui.DescriptionEditor.5a2bb2c9dc0e6bb5")} onMouseDown={event=>event.preventDefault()} onClick={()=>{setExpanded(value=>!value);editor.current?.focus();}}><Icon name={expanded?'close':'fullscreen'}/></button></Hint>
  </div>
  {linkOpen?<div className="description-editor__link"><Hint message={{id:"text.link-address"}}><input aria-label={localeText("details", "ui.DescriptionEditor.7ca51fc8c2925072")} placeholder={localeText("details", "ui.DescriptionEditor.9fefc3b55eb467e2")} value={link} onChange={event=>setLink(event.target.value)} autoFocus/></Hint><Hint message={{id:"text.link-apply"}}><button type="button" onClick={()=>{const safe=safeDescriptionLink(link);if(!safe){setError(localeText("details", "ui.DescriptionEditor.ee0a1ac17606d164"));return;}if(!range.current||range.current.collapsed){setError(localeText("details", "ui.DescriptionEditor.f2fc2657863cf783"));return;}command('createLink',safe);setLinkOpen(false);}}>{localeText("details", "ui.DescriptionEditor.190685f0a0774e64")}</button></Hint><Hint message={{id:"common.cancel-draft"}}><button type="button" aria-label={localeText("details", "ui.DescriptionEditor.d432380555a200c9")} onClick={()=>setLinkOpen(false)}><Icon name="close"/></button></Hint></div>:null}
  <div ref={editor} className="description-editor__body metadata-textarea" data-empty={empty} data-placeholder={localeText("details","description.placeholder")} role="textbox" aria-label={localeText("details", "ui.DescriptionEditor.58be531411a2f7b9")} aria-multiline="true" contentEditable={!disabled} aria-disabled={disabled} suppressContentEditableWarning onInput={publish} onMouseUp={remember} onKeyUp={remember}
   onPaste={event=>{event.preventDefault();command('insertText',event.clipboardData.getData('text/plain'));}} onDrop={event=>event.preventDefault()} onClick={event=>{if((event.target as HTMLElement).closest('a'))event.preventDefault();}}/>
  {error?<p role="alert">{error}</p>:null}
 </div>;
}
