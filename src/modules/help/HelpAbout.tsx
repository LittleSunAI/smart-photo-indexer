import packageInfo from '../../../package.json';
import notices from '../../../THIRD_PARTY_NOTICES.md?raw';
import projectLicense from '../../../LICENSE?raw';
import projectNotice from '../../../NOTICE?raw';
import {tourText} from './HelpRegistry';
import {useCurrentLocale} from '../../shared/i18n/Runtime';
import cover from '../../assets/about/cover-art.png';
import library from '../../assets/about/01-your-library.png';
import describe from '../../assets/about/02-describe-and-tag.png';
import search from '../../assets/about/03-search-by-words.png';
import edit from '../../assets/about/04-view-and-refine.png';
const illustrations=[library,describe,search,edit];
/** Local illustrated introduction; no README renderer or network request. */
export function HelpAbout({onOpenImage}:{onOpenImage?:(url:string)=>void}={}){
 useCurrentLocale();
 return <section className="help-about" data-testid="help-about">
  <div className="help-about__heading"><strong>Smart Photo Indexer</strong><span>{tourText('tour.about.version',{version:packageInfo.version})}</span></div>
  <img className="help-about__cover" src={cover} alt="" loading="lazy" decoding="async"/>
  <h2>{tourText('tour.about.tagline')}</h2>
  <p>{tourText('tour.about.lead')}</p>
  <div className="help-about__steps">{illustrations.map((url,index)=>{const prefix='tour.about.step'+(index+1),image=<img src={url} alt={tourText(prefix+'.alt')} loading="lazy" decoding="async"/>;return <section key={url} className="help-about__step"><h3>{index+1}. {tourText(prefix+'.title')}</h3>{onOpenImage?<button className="help-about__picture" type="button" aria-label={tourText(prefix+'.alt')} onClick={()=>onOpenImage(url)}>{image}</button>:image}<p>{tourText(prefix+'.body')}</p></section>;})}</div>
  <small className="help-about__examples">{tourText('tour.about.examples')}</small>
  <p>{tourText('tour.about.more')}</p>
  <p>{tourText('tour.about.requirements')}</p>
  <p className="help-about__license-status">{tourText('tour.about.license-pending')}</p>
  <div className="help-about__footer"><span>{tourText('tour.about.author',{author:'LittleSunAI'})}</span>
   <details className="help-about__licenses"><summary>{tourText('tour.about.license-text')}</summary><pre tabIndex={0}>{projectNotice+'\n'+projectLicense}</pre></details>
   <details className="help-about__licenses"><summary>{tourText('tour.about.third-party')}</summary><pre tabIndex={0}>{notices}</pre></details>
  </div>
 </section>;
}
