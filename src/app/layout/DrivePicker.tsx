import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useState} from 'react';
import {invoke} from "../../shared/tauri/invoke";
import {CompactSelect} from '../../shared/ui/CompactSelect';
import {isTauriRuntime} from '../../shared/tauri/runtime';
import {useAppDispatch} from '../state/AppStateContext';

export function DrivePicker({path,disabled,onChoose}:{path:string|null;disabled:boolean;onChoose(path:string):void}){
  useCurrentLocale();
  const [roots,setRoots]=useState<string[]>([]),dispatch=useAppDispatch();
  useEffect(()=>{
    let alive=true;
    const read=isTauriRuntime()?invoke<string[]>('get_filesystem_roots'):Promise.resolve(['C:\\','D:\\']);
    void read.then(value=>{if(alive)setRoots(value);}).catch(error=>{
      if(alive)dispatch({type:'app/setStatusMessage',message:localeText("main", "ui.DrivePicker.2b2cdeaa907fa007", {"value1": (String(error))}),severity:'error'});
    });
    return()=>{alive=false;};
  },[dispatch]);
  const root=path?.match(/^[a-z]:[\\/]/i)?.[0].replace('/','\\')??'';
  return <CompactSelect tourId="main.navigation.computer" helpId="computer-drives" icon="computer" caption={localeText("main", "ui.DrivePicker.9d693c610b4bef41")} className="drive-picker"
    value={root} disabled={disabled||!roots.length} options={roots.map(value=>({value,label:value.replace(/\\$/,'')}))}
    onChange={onChoose} message={{id:'main.computer'}}/>;
}
