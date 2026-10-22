import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useLayoutEffect,useRef,useState} from 'react';
import {convertFileSrc} from '@tauri-apps/api/core';
import type {EditorAsset} from './EditorContracts';
import {orientationTransform,type WorkingOrientation} from './EditorOrientation';

/** Fit the saved working orientation into the actual thumbnail area. */
export function EditorHistoryThumbnail({asset,orientation}:{asset:EditorAsset;orientation:WorkingOrientation}){
  useCurrentLocale();
 const root=useRef<HTMLDivElement>(null),[size,setSize]=useState({width:0,height:0}),[failed,setFailed]=useState(false);
 useLayoutEffect(()=>setFailed(false),[asset.previewPath]);
 useLayoutEffect(()=>{
  const element=root.current;if(!element)return;
  const update=()=>{const rotated=orientation.rotation%2,w=rotated?asset.height:asset.width,h=rotated?asset.width:asset.height,scale=Math.min(element.clientWidth/w,element.clientHeight/h),next={width:asset.width*scale,height:asset.height*scale};setSize(old=>old.width===next.width&&old.height===next.height?old:next);};
  const observer=new ResizeObserver(update);observer.observe(element);update();return()=>observer.disconnect();
 },[asset.width,asset.height,orientation.rotation]);
 return <div ref={root} className="editor-history-image">{failed?<span className="editor-muted">{localeText("editor", "ui.EditorHistoryThumbnail.ed7ff47239932e96")}</span>:<img draggable={false} loading="lazy" src={convertFileSrc(asset.previewPath)} alt="" onError={()=>setFailed(true)} style={{position:'absolute',left:'50%',top:'50%',width:size.width,height:size.height,transform:`translate(-50%,-50%) ${orientationTransform(orientation)}`}}/>}</div>;
}
