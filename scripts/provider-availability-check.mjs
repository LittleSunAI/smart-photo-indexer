import assert from 'node:assert/strict';import {build} from 'esbuild';import {mkdtemp,writeFile,rm} from 'node:fs/promises';import {tmpdir} from 'node:os';import {join,resolve,sep} from 'node:path';import {createRequire} from 'node:module';
const scratch=await mkdtemp(join(tmpdir(),'smart-photo-provider-gate-'));
try{const bundle=await build({stdin:{resolveDir:process.cwd(),loader:'ts',contents:`
import {isProviderAvailable,providerUnavailableMessage} from './src/modules/settings/ProviderAvailability';
import flag from './src/shared/config/provider-availability.json';
import {prepareSavedProvider} from './src/modules/settings/ProviderReadiness';
import {OpenRouterProvider} from './src/modules/ai/OpenRouterProvider';
import {openRouterCheckService} from './src/modules/openRouter/OpenRouterCheckService';
import {settingsService,defaultAppSettings} from './src/modules/settings/SettingsService';
import {i18n} from './src/shared/i18n/Runtime';import en from './src/shared/i18n/generated/en.json';import de from './src/shared/i18n/generated/de.json';import fr from './src/shared/i18n/generated/fr.json';
export async function check(){
 let calls=0;globalThis.fetch=async()=>{calls++;throw Error('Network forbidden in gate test');};
 if(isProviderAvailable('openrouter')!==flag.openrouter||!isProviderAvailable('fal')||!isProviderAvailable('local'))throw Error('Provider scope mismatch');
 if(defaultAppSettings.aiProviderMode!=='fal')throw Error('New-profile default must be Fal');
 const fast='qwen3-vl:2b-instruct-q4_K_M';
 if(defaultAppSettings.localAi.selectedModel!==fast||defaultAppSettings.localAi.selectedPresetId!==fast)throw Error('Fresh local mode must use the verified fast preset');
 for(const preset of ['qwen3.5:2b','qwen3.5:4b',fast,'invalid-fixture']){
  const saved=JSON.stringify({...defaultAppSettings,aiProviderMode:'local',localAi:{...defaultAppSettings.localAi,selectedPresetId:preset}});
  globalThis.window={localStorage:{getItem:key=>key==='smart-photo-indexer.settings.v1'?saved:null,setItem:()=>{throw Error('Must not rewrite preferences');}}};
  const restored=settingsService.loadSettings();if(restored.localAi.selectedModel!==(preset==='invalid-fixture'?fast:preset))throw Error('Explicit local choice lost or invalid fallback is not fast');
 }
 delete globalThis.window;

 const old=JSON.stringify({...defaultAppSettings,aiProviderMode:'openrouter',openRouter:{...defaultAppSettings.openRouter,apiKey:'fixture-legacy-key',credentialRef:'fixture-saved-ref'}});
 globalThis.window={localStorage:{getItem:key=>key==='smart-photo-indexer.settings.v1'?old:null,setItem:()=>{throw Error('Must not rewrite preferences');}}};
 const restored=settingsService.loadSettings();if(restored.aiProviderMode!=='openrouter'||restored.openRouter.apiKey!=='fixture-legacy-key'||restored.openRouter.credentialRef!=='fixture-saved-ref')throw Error('Saved preferences lost');delete globalThis.window;
 for(const [locale,resource]of Object.entries({en,de,fr}))for(const [ns,data]of Object.entries(resource))i18n.addResourceBundle(locale,ns,data);
 const messages=[];for(const locale of ['ru','en','de','fr']){await i18n.changeLanguage(locale);messages.push(providerUnavailableMessage());}
 if(new Set(messages).size!==4||messages.some(m=>m.includes('provider.openrouter-unavailable')))throw Error('Reason not localized');
 if(!flag.openrouter){const ready=await prepareSavedProvider('openrouter','unused','invalid-reference');if(ready.ready)throw Error('Readiness bypass');const checked=await openRouterCheckService.check('fixture','unused');if(checked.ok)throw Error('Legacy check bypass');let rejected=false;try{await new OpenRouterProvider({apiKey:'fixture',model:'unused'}).analyzeImage({});}catch(e){rejected=e.retryable===false;}if(!rejected)throw Error('Legacy inference bypass');}
 if(calls)throw Error('Network called');return {openrouterEnabled:flag.openrouter,localAndFalAvailable:true,preferencesPreserved:true,localizedReasons:messages.length,networkRequests:calls};}
`},bundle:true,platform:'node',format:'cjs',write:false,logLevel:'silent'});const file=join(scratch,'gate.cjs');await writeFile(file,bundle.outputFiles[0].text);const require=createRequire(import.meta.url);console.log(JSON.stringify(await require(file).check(),null,2));}
finally{assert(resolve(scratch).startsWith(resolve(tmpdir())+sep)&&scratch.split(sep).at(-1).startsWith('smart-photo-provider-gate-'));await rm(scratch,{recursive:true,force:true});}
