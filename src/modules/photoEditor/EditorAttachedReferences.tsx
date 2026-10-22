import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {useContext} from 'react';
import {EditorReferencesContext,ReferenceThumbnail} from './EditorReferenceControls';

/** Inventory includes stored samples of inactive marks, separately from the current request. */
export function EditorAttachedReferences(){
  useCurrentLocale();
 const context=useContext(EditorReferencesContext);
 if(!context)return null;
 const {draft,assets}=context;
 const groups=draft.annotations.map((a,i)=>({id:a.id,name:`${a.kind==='point'?localeText("editor", "ui.EditorAttachedReferences.d6f8648a7f6fa6f9"):a.kind==='rect'?localeText("editor", "ui.EditorAttachedReferences.c72503277b17588b"):localeText("editor", "ui.EditorAttachedReferences.c2cf53a21a833245")} ${i+1}`,items:draft.annotationReferences[a.id]??[]})).filter(g=>g.items.length);
 const count=draft.references.length+groups.reduce((n,g)=>n+g.items.length,0);
 return <details className="editor-attached-references"><Hint message={{id:"reference.all"}}><summary>{localeText("editor", "ui.EditorAttachedReferences.9305911819b2286b")}{' '}{count}</summary></Hint><div className="editor-attached-reference-list">
  {!count?<p className="editor-muted">{localeText("editor", "ui.EditorAttachedReferences.8dbb7994d0e7225a")}</p>:null}
  {draft.references.length?<section><strong>{localeText("editor", "ui.EditorAttachedReferences.c283d4bde600d159")}{' '}{draft.references.length}</strong><div className="editor-reference-thumbnails">{draft.references.map(r=>{const a=assets.find(a=>a.id===r.assetId);return a?<figure key={r.assetId}><ReferenceThumbnail asset={a}/></figure>:null;})}</div></section>:null}
  {groups.map(g=><section key={g.id}><Hint message={{id:"reference.all"}}><button type="button" disabled={context.busy} title={localeText("editor", "ui.EditorAttachedReferences.a94370a33a0d0a0a")} onClick={()=>window.dispatchEvent(new CustomEvent('editor-reveal-markup',{detail:g.id}))}>{g.name} · {g.items.length} {' '}{localeText("editor", "ui.EditorAttachedReferences.e27234567928892b")}</button></Hint><div className="editor-reference-thumbnails">{g.items.map(r=>{const a=assets.find(a=>a.id===r.assetId);return a?<figure key={r.assetId} className={r.included?'':'is-excluded'}><ReferenceThumbnail asset={a}/></figure>:null;})}</div></section>)}
 </div><p className="editor-muted">{localeText("editor", "ui.EditorAttachedReferences.1ba73e3f82b8440d")}</p></details>;
}
