import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useEffect,useRef,useState} from 'react';
import {Icon} from '../../shared/ui/Icon';
import {AnchoredPanel} from '../../shared/ui/AnchoredPanel';
export function FilenameFilter({value,onChange,context,loading}:{value:string;onChange(value:string):void;context:string;loading:boolean}){
  useCurrentLocale();
 const [open,setOpen]=useState(false),anchor=useRef<HTMLButtonElement>(null);useEffect(()=>setOpen(false),[context]);
 return <><Hint message={{id:"main.filename-filter"}}><button data-tour-id="main.display.filename" ref={anchor} type="button" className={`filename-filter${value?' is-active':''}`} aria-label={localeText("main", "ui.FilenameFilter.9cad8aa40d54554e")} title={value?localeText("main", "ui.FilenameFilter.91555babda20a50b", {"value1": (value)}):localeText("main", "ui.FilenameFilter.e9155c625c1ea4d7")} aria-haspopup="dialog" aria-expanded={open} onClick={()=>setOpen(v=>!v)}><Icon name="search"/>{value?<span className="filename-filter__dot"/>:null}</button></Hint>{open?<AnchoredPanel anchor={anchor} label={localeText("main", "ui.FilenameFilter.0d8dccec24fab43c")} onClose={()=>setOpen(false)}><div className="filename-filter__input"><Hint message={{id:"main.filename-filter"}}><input data-tour-id="main.display.filename" aria-label={localeText("main", "ui.FilenameFilter.9cad8aa40d54554e")} placeholder={localeText("main", "ui.FilenameFilter.773f6223fd53701f")} value={value} onChange={e=>onChange(e.target.value)}/></Hint><Hint message={{id:"main.filename-clear"}}><button type="button" aria-label={localeText("main", "ui.FilenameFilter.d5f94f78b78242ce")} onClick={()=>onChange('')}><Icon name="close"/></button></Hint></div>{loading?<small role="status">{localeText("main", "ui.FilenameFilter.45be4113f7499662")}</small>:null}</AnchoredPanel>:null}</>;
}
