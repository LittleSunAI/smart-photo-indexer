import{useState}from'react';
import{text,useCurrentLocale}from'../../shared/i18n/Runtime';
import{useUiPreferences}from'../../shared/useUiPreferences';
import{saveUiPreferences}from'../../shared/UiPreferences';
import{changeTheme}from'../../shared/ThemeController';
export function AppearanceSettings(){useCurrentLocale();const p=useUiPreferences(),[error,setError]=useState('');return <div className="settings-appearance" data-tour-id="settings.general.appearance"><label className="settings-field"><span>{text('settings','tabs.theme')}</span><select aria-label={text('settings','tabs.theme')} value={p.theme} onChange={e=>{setError('');void changeTheme(e.target.value as'light'|'dark').catch(e=>setError(String(e)));}}><option value="light">{text('settings','tabs.light')}</option><option value="dark">{text('settings','tabs.dark')}</option></select></label><label className="settings-toggle"><input type="checkbox" checked={p.viewerOnlyPhoto} onChange={e=>{setError('');try{saveUiPreferences({viewerOnlyPhoto:e.target.checked});}catch(e){setError(String(e));}}}/>{text('settings','tabs.viewerOnlyPhoto')}</label>{error?<p role="alert">{error}</p>:null}</div>;}
