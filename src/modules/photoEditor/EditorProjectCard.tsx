import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useState} from 'react';
import {EditorHistoryThumbnail} from './EditorHistoryThumbnail';
import {Icon} from '../../shared/ui/Icon';
import type {EditorAsset} from './EditorContracts';
import type {WorkingOrientation} from './EditorOrientation';
import './editor-project-card.css';

export interface EditorProjectSummary {id:string;title:string;sourceName:string;updatedAtUnix:number}
export interface EditorProjectPreview {projectId:string;total:number;offset:number;frames:Array<{id:string;title:string;asset:EditorAsset;orientation:WorkingOrientation}>}

/** Reading a card never selects a project or writes its draft. */
export function EditorProjectCard({project,loadPreview,busy=false,active=false,onOpen,onDelete,paged=false}:{project:EditorProjectSummary;loadPreview(id:string,offset:number):Promise<EditorProjectPreview>;busy?:boolean;active?:boolean;onOpen():void;onDelete?():void;paged?:boolean}){
  useCurrentLocale();
 const [open,setOpen]=useState(false),[preview,setPreview]=useState<EditorProjectPreview|null>(null),[offset,setOffset]=useState(0),[error,setError]=useState(''),[loading,setLoading]=useState(false);
 useEffect(()=>{let disposed=false;setLoading(true);setError('');void loadPreview(project.id,offset).then(value=>{if(!disposed)setPreview(value);}).catch(e=>{if(!disposed)setError(String(e));}).finally(()=>{if(!disposed)setLoading(false);});return()=>{disposed=true;};},[loadPreview,project.id,project.updatedAtUnix,offset]);
 const cover=preview?.frames.at(-1);
 return <details className="editor-project-card" open={open} onToggle={event=>setOpen(event.currentTarget.open)}>
  <Hint message={{id:"project.card"}}><summary><span className="editor-project-cover">{cover?<EditorHistoryThumbnail asset={cover.asset} orientation={cover.orientation}/>:<Icon name="image"/>}</span><span className="editor-project-card__title"><strong title={project.title}>{project.title}</strong><small>{active?localeText("editor", "ui.EditorProjectCard.f646bd81717c2db8"):''}{new Date(project.updatedAtUnix*1000).toLocaleString(displayLocaleTag())}{preview?localeText("editor", "ui.EditorProjectCard.2f7893a2fe43e249", {"value1": (preview.total)}):''}</small></span>{onDelete?<Hint message={{id:"project.delete"}}><button type="button" className="editor-project-delete" title={localeText("editor", "ui.EditorProjectCard.87397879d2a45961")} aria-label={localeText("editor", "ui.EditorProjectCard.1b5107f32dcdbd07")} disabled={busy} onClick={event=>{event.preventDefault();event.stopPropagation();onDelete();}}><Icon name="trash"/></button></Hint>:null}<Hint message={{id:"project.open"}}><button type="button" disabled={busy} onClick={event=>{event.preventDefault();event.stopPropagation();onOpen();}}>{localeText("editor", "ui.EditorProjectCard.c8a862491be9f37a")}</button></Hint><Icon name={open?'up':'down'}/></summary></Hint>
  {open?<div className="editor-project-card__body">
   {error?<span role="status">{error}</span>:null}
   {preview?<><div className="editor-project-mini-strip" aria-label={localeText("editor", "ui.EditorProjectCard.c846d93b24f7d412", {"value1": (project.title)})}>{preview.frames.map(frame=><figure key={frame.id} title={frame.title}><EditorHistoryThumbnail asset={frame.asset} orientation={frame.orientation}/><figcaption>{frame.title}</figcaption></figure>)}</div>
   {paged&&preview.total>30?<div className="editor-project-card__actions"><Hint message={{id:"common.page-prev",args:{список:localeText("editor", "ui.EditorProjectCard.c96d299be6b180f9")}}}><button type="button" disabled={loading||offset===0} onClick={()=>setOffset(Math.max(0,offset-30))}>{localeText("editor", "ui.EditorProjectCard.26919c1669fadd6b")}</button></Hint><span>{offset+1}–{Math.min(preview.total,offset+preview.frames.length)} {' '}{localeText("editor", "ui.EditorProjectCard.6cd3e13df4cf74c1")}{' '}{preview.total}</span><Hint message={{id:"common.page-next",args:{список:localeText("editor", "ui.EditorProjectCard.c96d299be6b180f9")}}}><button type="button" disabled={loading||offset+30>=preview.total} onClick={()=>setOffset(offset+30)}>{localeText("editor", "ui.EditorProjectCard.5f76e312e6b2863a")}</button></Hint></div>:null}</>:loading?<span>{localeText("editor", "ui.EditorProjectCard.049fe0924057258b")}</span>:null}
  </div>:null}
 </details>;
}
