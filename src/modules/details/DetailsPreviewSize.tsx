import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {useEffect,useState,type CSSProperties,type ReactNode} from 'react';
import {useUiPreferences} from '../../shared/useUiPreferences';
import {saveUiPreferences} from '../../shared/UiPreferences';
import {EditorSplitter} from '../photoEditor/EditorSplitter';

/** Height belongs to the panel, independent of the selected photo and its metadata. */
export function DetailsPreviewSize({children}:{children:ReactNode}){
  useCurrentLocale();
 const preferences=useUiPreferences(),[height,setHeight]=useState(preferences.detailsPreviewHeight),[error,setError]=useState('');
 useEffect(()=>setHeight(preferences.detailsPreviewHeight),[preferences.detailsPreviewHeight]);
 const value=height??Math.max(128,Math.min(220,Math.round(innerHeight*.24)));
 return <div className="details-preview-size" style={{'--details-preview-height':`${value}px`} as CSSProperties}>{children}<EditorSplitter label={localeText("details", "ui.DetailsPreviewSize.fd5929cf7ab01d92")} axis="y" value={value} min={96} max={640} className="details-preview-splitter" onChange={setHeight} onCommit={next=>{try{saveUiPreferences({detailsPreviewHeight:next});setError('');}catch(e){setError(localeText("details", "ui.DetailsPreviewSize.df34e307c6fb09e8")+String(e));}}}/>{error?<span role="alert">{error}</span>:null}</div>;
}
