import {text,useCurrentLocale} from '../../shared/i18n/Runtime';
import {localeController,useLocaleState} from '../../shared/i18n/LocaleController';
import {availableLocales,localeNames} from '../../shared/i18n/Locale';
import {Hint} from '../../shared/help/Hint';
export function LanguageSetting(){
 useCurrentLocale();const state=useLocaleState();
 return <div className="language-setting" data-help-id="settings-language">
  <Hint message={{id:'settings.language'}}><label className="settings-field language-setting__row"><span>{text('settings','language.label')}</span><select data-testid="settings-language" aria-label={text('settings','language.aria')} aria-busy={state.pending!==null} value={state.pending??state.locale} onChange={event=>{void localeController.request(event.target.value);}}>{availableLocales.map(locale=><option key={locale} value={locale}>{localeNames[locale]}</option>)}</select></label></Hint>
  {state.error?<p className="settings-muted" role="alert">{text('settings',`language.error-${state.error}`)}</p>:null}
 </div>;
}
