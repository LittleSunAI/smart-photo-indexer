import {attachReadingWheel} from './ReadingWheel';
import {HelpAbout} from './HelpAbout';
import {Hint} from '../../shared/help/Hint';
import {HelpText} from '../../shared/help/HelpText';
import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import {createPortal} from 'react-dom';
import {Icon} from '../../shared/ui/Icon';
import {getHelpArticles,getHelpFeatures,getHelpGroups,helpContext,tourText,type HelpFeature} from './HelpRegistry';
import {readHelpProgress,writeHelpProgress,type HelpProgress} from './HelpProgress';
import {setHelpOpen,setHelpStep} from './HelpPresentation';
import {useCurrentLocale} from '../../shared/i18n/Runtime';
import {helpIllustrations} from './HelpIllustrations';
import {helpTourLayout} from './HelpTourLayout';
import './help.css';

function targets(feature:HelpFeature,primaryOnly=false){
 const collect=(ids:string[])=>ids.flatMap(id=>{
  const nodes=[...document.querySelectorAll<HTMLElement>(`[data-tour-id~="${CSS.escape(id)}"],[data-help-id="${CSS.escape(id)}"]`)].filter(node=>node.getClientRects().length&&getComputedStyle(node).visibility!=='hidden'&&getComputedStyle(node).display!=='none');
  // Dynamic lists explicitly mark the currently inspected record; never pick an unrelated first row.
  const current=nodes.filter(node=>node.dataset.tourCurrent==='true');return current.length?current:feature.targetMode==='union'?nodes:nodes.length===1?nodes:[];
 });
 const primary=collect(feature.targetIds);return primary.length||primaryOnly?primary:collect(feature.fallbackTargetIds??[]);
}
function clipped(node:HTMLElement){
 const r=node.getBoundingClientRect();let left=Math.max(4,r.left),right=Math.min(innerWidth-4,r.right),top=Math.max(4,r.top),bottom=Math.min(innerHeight-4,r.bottom);
 for(let parent=node.parentElement;parent;parent=parent.parentElement){const style=getComputedStyle(parent),box=parent.getBoundingClientRect();if(/hidden|auto|scroll|clip/.test(style.overflowX)){left=Math.max(left,box.left);right=Math.min(right,box.right);}if(/hidden|auto|scroll|clip/.test(style.overflowY)){top=Math.max(top,box.top);bottom=Math.min(bottom,box.bottom);}}
 return right>left&&bottom>top?{left,top,width:right-left,height:bottom-top}:null;
}
function bounds(feature:HelpFeature){const rects=targets(feature).map(clipped).filter((rect):rect is NonNullable<typeof rect>=>Boolean(rect));if(!rects.length)return null;const left=Math.min(...rects.map(r=>r.left)),top=Math.min(...rects.map(r=>r.top));return{left,top,width:Math.max(...rects.map(r=>r.left+r.width))-left,height:Math.max(...rects.map(r=>r.top+r.height))-top};}

export default function HelpCenter({onClose,initialGroup,initialFeature,initialArticle}:{onClose():void;initialGroup?:string;initialFeature?:string;initialArticle?:string}){
 const locale=useCurrentLocale(),context=helpContext(),helpFeatures=getHelpFeatures(),helpGroups=getHelpGroups(),helpArticles=getHelpArticles();
 const initial=useRef(readHelpProgress(helpFeatures,helpGroups)),requested=initialGroup??initial.current.steps[context]?.scope;
 const groups=helpGroups.filter(group=>group.context===context);
 const [groupId,setGroup]=useState(requested&&groups.some(group=>group.id===requested)?requested:'all');
 // This is the only order: filtering a group never sorts or concatenates it.
 const features=helpFeatures.filter(feature=>feature.context===context&&(groupId==='all'||feature.groupId===groupId));
 const [tab,setTab]=useState<HelpProgress['tab']>(initialArticle?'reference':initialGroup||initialFeature?'tour':initial.current.tab),[articleId,setArticle]=useState(initialArticle??(context==='editor'?'editor':context==='viewer'?'viewer':initial.current.articleId)),[query,setQuery]=useState('');
 const [stepId,setStep]=useState<string|null>(null),[rect,setRect]=useState<ReturnType<typeof bounds>>(null),[notice,setNotice]=useState(initial.current.stale?tourText('tour.ui.old-progress'):''),[zoomImage,setZoomImage]=useState<string|null>(null);
 const zoomReturnFocus=useRef<HTMLElement|null>(null);
 const openImage=(url:string)=>{zoomReturnFocus.current=document.activeElement as HTMLElement|null;setZoomImage(url);};
 const panel=useRef<HTMLDivElement>(null),zoomPanel=useRef<HTMLDivElement>(null),step=features.find(feature=>feature.id===stepId),index=features.findIndex(feature=>feature.id===stepId);
 const [cardSize,setCardSize]=useState({left:0,top:0,width:480,height:420}),[viewport,setViewport]=useState({width:innerWidth,height:innerHeight});
 const latest=useRef({onClose,stepId,zoomImage});latest.current={onClose,stepId,zoomImage};
 const last=initial.current.steps[context],normalized=query.toLocaleLowerCase(locale).trim();
 const filtered=helpArticles.filter(article=>(article.title+' '+article.aliases.join(' ')+' '+article.paragraphs.join(' ')).toLocaleLowerCase(locale).includes(normalized));
 const article=filtered.find(article=>article.id===articleId)??filtered[0];
 const persist=(change:Partial<HelpProgress>)=>{initial.current={...initial.current,...change};if(!writeHelpProgress(initial.current,context))setNotice(tourText('tour.ui.progress-save-error'));};
 const selectTab=(value:HelpProgress['tab'])=>{setTab(value);persist({tab:value});};
 const begin=(id:string,scope=groupId)=>{const feature=helpFeatures.find(feature=>feature.id===id&&feature.context===context);if(!feature)return;setGroup(scope);setZoomImage(null);setStep(id);persist({steps:{...initial.current.steps,[context]:{stepId:id,scope}}});};
 const startGroup=(scope:string)=>{const first=helpFeatures.find(feature=>feature.context===context&&(scope==='all'||feature.groupId===scope));if(first)begin(first.id,scope);};
 const end=()=>{setStep(null);setRect(null);};
 const showArticle=(id:string)=>{setQuery('');setArticle(id);persist({articleId:id,tab:'reference'});setTab('reference');end();};
 useEffect(()=>{const feature=initialFeature?helpFeatures.find(feature=>feature.id===initialFeature&&feature.context===context):undefined;if(feature)begin(feature.id,initialGroup??feature.groupId);else if(initialGroup)begin(helpFeatures.find(feature=>feature.context===context&&feature.groupId===initialGroup)?.id??'',initialGroup);},[]);
 useLayoutEffect(()=>{
  const owners=panel.current?.querySelectorAll<HTMLElement>('.help-reference article,.help-reference aside');
  const cleanup=[...(owners??[])].map(attachReadingWheel);
  return()=>cleanup.forEach(dispose=>dispose());
 },[tab,article?.id,stepId,locale]);
 useLayoutEffect(()=>{panel.current?.querySelector<HTMLElement>('.help-tabs [aria-selected="true"]')?.scrollIntoView({block:'nearest',inline:'nearest'});},[tab,locale]);
 useLayoutEffect(()=>{const update=()=>setViewport({width:innerWidth,height:innerHeight});window.addEventListener('resize',update);return()=>window.removeEventListener('resize',update);},[]);
 useLayoutEffect(()=>{if(!step||!panel.current)return;const node=panel.current,measure=()=>{const box=node.getBoundingClientRect();setCardSize(old=>old.width===box.width&&old.height===box.height?old:{left:0,top:0,width:box.width,height:box.height});};measure();const observer=new ResizeObserver(measure);observer.observe(node);return()=>observer.disconnect();},[stepId]);
 // One ownership interval for root inert, keyboard and focus. Next/Back do not release it.
 useLayoutEffect(()=>{
  const root=document.getElementById('root'),oldInert=root?.inert,focus=document.activeElement as HTMLElement|null;
  setHelpOpen(true);if(root)root.inert=true;document.documentElement.dataset.helpActive='true';
  const keys=(event:KeyboardEvent)=>{
   const current=latest.current,host=current.zoomImage?zoomPanel.current:panel.current;
   if(event.key==='Escape'){event.preventDefault();event.stopImmediatePropagation();if(current.zoomImage)setZoomImage(null);else if(current.stepId)end();else current.onClose();return;}
   if(event.key==='Tab'){
    const controls=[...(host?.querySelectorAll<HTMLElement>('button:not(:disabled),input,select,a[href],[tabindex="0"]')??[])].filter(node=>node.getClientRects().length);
    if(controls.length){const at=controls.indexOf(document.activeElement as HTMLElement);if(at<0||(event.shiftKey&&at===0)||(!event.shiftKey&&at===controls.length-1)){event.preventDefault();controls[event.shiftKey?controls.length-1:0].focus();}}
   }
   const inside=Boolean(event.target&&host?.contains(event.target as Node)),copy=event.ctrlKey&&['c','a'].includes(event.key.toLowerCase());
   if(!inside||event.altKey||event.metaKey||(event.ctrlKey&&!copy))event.preventDefault();event.stopPropagation();
  };
  window.addEventListener('keydown',keys,true);
  const modal=()=>{if(document.querySelector('#root [role="dialog"][aria-modal="true"]:not([data-help-overlay="settings"]),dialog[open]'))latest.current.onClose();};
  const observer=new MutationObserver(modal);if(root)observer.observe(root,{subtree:true,childList:true,attributes:true,attributeFilter:['open','aria-modal']});
  return()=>{observer.disconnect();window.removeEventListener('keydown',keys,true);setHelpOpen(false);if(root)root.inert=oldInert??false;delete document.documentElement.dataset.helpActive;delete document.documentElement.dataset.helpStep;if(focus?.isConnected)focus.focus({preventScroll:true});};
 },[]);
 useLayoutEffect(()=>{
  setHelpStep(step?.id??null,step?.presentation);if(!step){setRect(null);delete document.documentElement.dataset.helpStep;return;}
  document.documentElement.dataset.helpStep=step.id;let frame=0;const observed=new Set<HTMLElement>();
  const update=()=>{frame=0;for(const node of targets(step)){if(!observed.has(node)){observed.add(node);node.scrollIntoView({block:'nearest',inline:'nearest',behavior:'instant'});resize.observe(node);}}const next=bounds(step);setRect(old=>JSON.stringify(old)===JSON.stringify(next)?old:next);};
  const schedule=()=>{if(!frame)frame=requestAnimationFrame(update);};
  const resize=new ResizeObserver(schedule);resize.observe(document.documentElement);
  // Compatibility notification contains presentation metadata only. No navigation bridge/actions.
  window.dispatchEvent(new CustomEvent('smart-photo-indexer:help-reveal',{detail:step}));schedule();
  const observer=new MutationObserver(schedule);const root=document.getElementById('root');if(root)observer.observe(root,{childList:true,subtree:true,attributes:true,attributeFilter:['hidden','open','class']});
  window.addEventListener('resize',schedule);window.addEventListener('scroll',schedule,true);
  return()=>{cancelAnimationFrame(frame);resize.disconnect();observer.disconnect();window.removeEventListener('resize',schedule);window.removeEventListener('scroll',schedule,true);};
 },[stepId,locale]);
 useLayoutEffect(()=>{const frame=requestAnimationFrame(()=>{const host=zoomImage?zoomPanel.current:panel.current,back=zoomReturnFocus.current;if(!zoomImage&&back?.isConnected&&host?.contains(back)){zoomReturnFocus.current=null;back.focus({preventScroll:true});}else if(!host?.contains(document.activeElement))host?.querySelector<HTMLElement>('button,input,select')?.focus();});return()=>cancelAnimationFrame(frame);},[stepId,zoomImage,tab]);
 const closeButton=<Hint message={{id:'common.close-dialog'}}><button type="button" aria-label={tourText('tour.ui.close')} className="help-close" onClick={onClose}><Icon name="close"/></button></Hint>;
 const picture=article?helpIllustrations.find(picture=>picture.articleId===article.id):undefined;
 const tour=step?(()=>{const {left,top,arrow,maxHeight}=helpTourLayout(rect,cardSize,viewport),nodes=targets(step),reason=nodes.map(node=>node.dataset.tourUnavailable).find(Boolean),disabled=Boolean(reason)||nodes.some(node=>node.matches(':disabled')||node.closest('fieldset:disabled')||node.tagName==='LABEL'&&node.querySelector('input,select,button,textarea')?.matches(':disabled'));return <div className="help-tour-blocker" role="presentation">
  {rect?<><div className="help-spotlight" data-tour-step={step.id} style={{left:rect.left-3,top:rect.top-3,width:rect.width+6,height:rect.height+6}}/>{arrow?<svg className="help-arrow" aria-hidden="true" width="100%" height="100%"><defs><marker id="help-arrow-head" markerWidth="7" markerHeight="7" refX="6" refY="3.5" orient="auto"><path d="M0 0L7 3.5L0 7" fill="var(--blue)"/></marker></defs><path d={arrow} stroke="var(--blue)" strokeWidth="2" markerEnd="url(#help-arrow-head)"/></svg>:null}</>:null}
  <div ref={panel} role="dialog" aria-modal="true" aria-label={tourText('tour.ui.tour-tab')} data-tour-card={step.id} className="help-tour-card" style={{left,top,maxHeight}}><header><span>{groupId==='all'?tourText('tour.ui.all-groups'):groups.find(group=>group.id===groupId)?.title} · {index+1}/{features.length}</span>{closeButton}</header><strong>{step.title}</strong><HelpText message={step.message}/>
   {!rect?<p role="status" className="help-unavailable">{tourText('tour.ui.not-visible')} {step.unavailable||tourText('tour.ui.target-missing')}</p>:!targets(step,true).length?<p className="help-unavailable">{step.unavailable||tourText('tour.ui.condition-followup')}</p>:disabled?<p className="help-unavailable">{tourText('tour.ui.disabled')} {reason?tourText(reason):step.unavailable||tourText('tour.ui.condition-followup')}</p>:null}
   <Hint message={{id:'help.feature'}}><label>{tourText('tour.ui.step-label')}<select aria-label={tourText('tour.ui.step-label')} value={step.id} onChange={event=>begin(event.target.value)}>{features.map(feature=><option key={feature.id} value={feature.id}>{feature.title}</option>)}</select></label></Hint>
   <div className="help-tour-group"><Hint message={{id:'help.group'}}><label>{tourText('tour.ui.group-label')}<select aria-label={tourText('tour.ui.group-label')} value={groupId} onChange={event=>startGroup(event.target.value)}><option value="all">{tourText('tour.ui.all-groups')}</option>{groups.map(group=><option key={group.id} value={group.id}>{group.title}</option>)}</select></label></Hint></div>
   <footer><Hint message={{id:'help.groups'}}><button type="button" className="help-groups" onClick={end}>{tourText('tour.ui.to-groups')}</button></Hint><Hint message={{id:'help.step-prev'}}><button type="button" className="help-prev" disabled={index===0} onClick={()=>begin(features[index-1].id)}>{tourText('tour.ui.previous')}</button></Hint><Hint message={{id:'help.reference-tab'}}><button type="button" className="help-reference-link" onClick={()=>showArticle(step.articleId)}>{tourText('tour.ui.to-reference')}</button></Hint><Hint message={{id:'help.step-next'}}><button type="button" className="help-next" onClick={()=>index===features.length-1?end():begin(features[index+1].id)}>{tourText(index===features.length-1?'tour.ui.finish':'tour.ui.next')}</button></Hint></footer>{notice?<small role="status">{notice}</small>:null}
  </div></div>;})():null;
 return createPortal(tour??<div className="help-overlay"><div ref={panel} role="dialog" aria-modal="true" aria-label={tourText('tour.ui.title')} className="help-center"><header><strong>{tourText('tour.ui.title')}</strong>{closeButton}</header><nav className="help-tabs" role="tablist" aria-label={tourText('tour.ui.title')}>{(['about','tour','reference'] as const).map(value=><button key={value} id={'help-tab-'+value} type="button" role="tab" aria-selected={tab===value} tabIndex={tab===value?0:-1} aria-controls={'help-panel-'+value} onKeyDown={event=>{const tabs=['about','tour','reference'] as const;if(!['ArrowLeft','ArrowRight','Home','End'].includes(event.key))return;event.preventDefault();const at=tabs.indexOf(value),next=event.key==='Home'?0:event.key==='End'?2:(at+(event.key==='ArrowRight'?1:2))%3;selectTab(tabs[next]);event.currentTarget.parentElement?.querySelector<HTMLButtonElement>('#help-tab-'+tabs[next])?.focus();}} onClick={()=>selectTab(value)}>{tourText('tour.ui.'+(value==='about'?'about-tab':value==='tour'?'tour-tab':'reference-tab'))}</button>)}</nav>
  {tab==='about'?<div role="tabpanel" id="help-panel-about" aria-labelledby="help-tab-about" className="help-about-scroll"><HelpAbout onOpenImage={openImage}/></div>:tab==='tour'?<div role="tabpanel" id="help-panel-tour" aria-labelledby="help-tab-tour" className="help-tour-topics"><p>{tourText('tour.ui.intro')}</p><div className="help-start-actions"><Hint message={{id:'help.all'}}><button type="button" onClick={()=>startGroup('all')}>{tourText('tour.ui.start-all')}</button></Hint>{last?<Hint message={{id:'help.continue'}}><button type="button" onClick={()=>begin(last.stepId,last.scope)}>{tourText('tour.ui.continue')}</button></Hint>:null}</div>
   {groups.map(group=><details className="help-topic-group" key={group.id}><summary><strong>{group.title}</strong><span>{helpFeatures.filter(feature=>feature.context===context&&feature.groupId===group.id).length}</span><Icon name="down"/></summary><div className="help-topic-group__body"><p>{group.description}</p><Hint message={{id:'help.group-start'}}><button type="button" className="help-start-group" onClick={()=>startGroup(group.id)}>{tourText('tour.ui.start-group')}</button></Hint>{helpFeatures.filter(feature=>feature.context===context&&feature.groupId===group.id).map(feature=><button type="button" className="help-topic-step" key={feature.id} onClick={()=>begin(feature.id,group.id)}><strong>{feature.title}</strong><span>{feature.explanation}</span></button>)}</div></details>)}
   <p className="help-other-windows">{tourText('tour.ui.other-windows')}</p></div>:<div role="tabpanel" id="help-panel-reference" aria-labelledby="help-tab-reference" className="help-reference"><aside tabIndex={0} data-testid="help-reading-list" aria-label={tourText('tour.ui.reference-tab')}><input aria-label={tourText('tour.ui.reference-search')} placeholder={tourText('tour.ui.reference-search')} value={query} onChange={event=>setQuery(event.target.value)}/><select className="help-mobile-articles" aria-label={tourText('tour.ui.reference-tab')} value={article?.id??''} onChange={event=>{setArticle(event.target.value);persist({articleId:event.target.value});}}>{filtered.map(article=><option key={article.id} value={article.id}>{article.title}</option>)}</select><nav className="help-article-list" aria-label={tourText('tour.ui.reference-tab')}>{filtered.map(article=><button type="button" key={article.id} aria-current={article.id===articleId?'page':undefined} onClick={()=>{setArticle(article.id);persist({articleId:article.id});}}>{article.title}</button>)}</nav></aside>
   {article?<article key={article.id} tabIndex={0} data-testid="help-reading-article" aria-label={article.title}><h2>{article.title}</h2>{article.paragraphs.map((paragraph,i)=><p key={i}>{paragraph}</p>)}{picture?<button type="button" className="help-illustration" aria-label={tourText('tour.ui.picture-open')} onClick={()=>openImage(picture.url)}><img src={picture.url} alt={picture.caption} loading="lazy"/><span>{picture.caption}</span></button>:null}<div className="help-article-actions">{helpFeatures.filter(feature=>feature.articleId===article.id).map(feature=>feature.context===context?<button type="button" key={feature.id} onClick={()=>begin(feature.id,feature.groupId)}>{tourText('tour.ui.reference-show',{название:feature.title})}</button>:<span key={feature.id}>{tourText(feature.context==='editor'?'tour.ui.reference-in-editor':feature.context==='viewer'?'tour.ui.reference-in-viewer':'tour.ui.reference-in-main',{название:feature.title})}</span>)}</div></article>:<article className="help-empty" role="status"><h2>{tourText('tour.ui.reference-empty-title')}</h2><p>{tourText('tour.ui.reference-empty')}</p></article>}</div>}{notice?<p role="status" className="help-notice">{notice}</p>:null}
  {zoomImage?<div ref={zoomPanel} className="help-image-view" role="dialog" aria-modal="true" aria-label={tourText('tour.ui.picture-open')}><button type="button" aria-label={tourText('tour.ui.picture-close')} onClick={()=>setZoomImage(null)}><Icon name="close"/></button><img src={zoomImage} alt={tourText('tour.ui.picture-open')}/></div>:null}
 </div></div>,document.body);
}
