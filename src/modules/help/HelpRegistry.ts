import routes from './TourRoutes.json';
import groups from './TourGroups.json';
import {i18n,richMessage} from '../../shared/i18n/Runtime';
import {tourMessage} from './TourMessages';
import type {HelpMessage} from '../../shared/help/HelpMessage';
import type {TourPresentation} from './HelpPresentation';
import type {RichToken} from '../../shared/i18n/Runtime';
export type HelpContext='main'|'editor'|'viewer';
export interface HelpArticle{id:string;title:string;aliases:string[];paragraphs:string[]}
export interface HelpGroup{id:string;context:HelpContext;title:string;description:string}
export interface HelpFeature{id:string;articleId:string;context:HelpContext;groupId:string;title:string;explanation:string;unavailable:string;message:HelpMessage;targetIds:string[];targetMode?:'union';fallbackTargetIds?:string[];presentation:TourPresentation}
export function tourText(key:string,args:Record<string,string|number>={}):string{return i18n.t(key,{...args,ns:'tour'}) as string;}
function record(ns:string,key:string){return i18n.getResource(i18n.resolvedLanguage??'ru',ns,key)??i18n.getResource('ru',ns,key);}
function plain(tokens:RichToken[]):string{return tokens.map(token=>typeof token==='string'?token:token.kind==='arg'?'':plain(token.children)).join('');}
export function getHelpFeatures():HelpFeature[]{
 const current=(routes as {id:string;context:HelpContext;groupId:string;targetIds:string[];articleId:string;unavailableIds:string[];presentation:TourPresentation}[]).map(route=>{
  const message=tourMessage(route.id);return {...route,message,title:tourText(route.id+'.title'),explanation:richMessage(message.id).map(plain).join(' '),unavailable:route.unavailableIds.map(id=>tourText(id)).join(' ')};
 });
 return current;
}
export function getHelpGroups():HelpGroup[]{
 return [...groups.map(group=>({id:group.id,context:group.context as HelpContext,title:tourText(group.id+'.title'),description:plain(richMessage(tourMessage(group.overviewId).id)[0])}))];
}
export function getHelpArticles():HelpArticle[]{
 const source=i18n.getResourceBundle(i18n.resolvedLanguage??'ru','reference')??i18n.getResourceBundle('ru','reference');
 return Object.keys(source).filter(key=>key.startsWith('helpArticles.')).map(key=>({id:key.slice('helpArticles.'.length),...record('reference',key)}));
}
export function featureGroup(feature:HelpFeature){return feature.groupId;}
export function helpContext():HelpContext{const p=new URLSearchParams(location.search);return p.get('window')==='editor'?'editor':p.get('window')==='viewer'||p.get('viewer')==='1'||p.has('viewerLabel')?'viewer':'main';}
