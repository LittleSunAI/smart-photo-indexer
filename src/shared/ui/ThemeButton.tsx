import {text as localeText, useCurrentLocale} from "../i18n/Runtime";
import {Hint} from "../help/Hint";
import {useState} from 'react';
import {useUiPreferences} from '../useUiPreferences';
import {changeTheme} from '../ThemeController';
import {Icon} from './Icon';
export function ThemeButton(){
  useCurrentLocale();const {theme}=useUiPreferences(),[error,setError]=useState('');const title=theme==='dark'?localeText("common", "ui.ThemeButton.b0cb84985cd04c32"):localeText("common", "ui.ThemeButton.33eabf52eaf64f67");return <span className="appearance-control"><Hint message={{id:theme==='dark'?'common.theme-light':'common.theme-dark'}}><button type="button" data-help-id="appearance" data-tour-id="main.commands.theme" aria-label={title} title={title} onClick={()=>{setError('');void changeTheme(theme==='dark'?'light':'dark').catch(e=>setError(String(e)));}}><Icon name={theme==='dark'?'sun':'moon'}/></button></Hint>{error?<span className="appearance-error" role="alert">{error}<Hint message={{id:"common.dismiss-message"}}><button type="button" aria-label={localeText("common", "ui.ThemeButton.f35e494ccb7d76f2")} onClick={()=>setError('')}>×</button></Hint></span>:null}</span>;}
