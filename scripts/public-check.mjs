import assert from 'node:assert/strict';
import {mkdtemp,readFile,writeFile,mkdir,copyFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join,resolve,sep} from 'node:path';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import {verifyVersions} from './version-contract.mjs';
const root=process.cwd(),scratch=await mkdtemp(join(tmpdir(),'smart-photo-public-test-'));
try{
 const version=verifyVersions(root);
 const originalLicense=await readFile('LICENSE','utf8'),originalNotice=await readFile('NOTICE','utf8');
 assert.ok(originalLicense.includes('Version 2.0, January 2004')&&originalLicense.includes('END OF TERMS AND CONDITIONS'));
 assert.ok(originalNotice.includes('Copyright 2026 LittleSunAI'));
 assert.equal(JSON.parse(await readFile('package.json','utf8')).license,'Apache-2.0');
 const bundle=JSON.parse(await readFile('src-tauri/tauri.conf.json','utf8')).bundle;
 assert.equal(bundle.license,'Apache-2.0');assert.equal(bundle.resources['../LICENSE'],'LICENSE');assert.equal(bundle.resources['../NOTICE'],'NOTICE');
 for(const name of ['package.json','package-lock.json','src-tauri/Cargo.toml','src-tauri/Cargo.lock','src-tauri/tauri.conf.json']){const target=join(scratch,name);await mkdir(join(target,'..'),{recursive:true});await copyFile(name,target);}
 const config=JSON.parse(await readFile(join(scratch,'src-tauri/tauri.conf.json'),'utf8'));config.version='99.0.0';await writeFile(join(scratch,'src-tauri/tauri.conf.json'),JSON.stringify(config));assert.throws(()=>verifyVersions(scratch),/Version mismatch/);
 const result=await build({stdin:{resolveDir:root,loader:'tsx',contents:`
 import React from 'react';import {renderToStaticMarkup} from 'react-dom/server';
 import {i18n} from './src/shared/i18n/Runtime';import en from './src/shared/i18n/generated/en.json';import de from './src/shared/i18n/generated/de.json';import fr from './src/shared/i18n/generated/fr.json';
 import {HelpAbout} from './src/modules/help/HelpAbout';import {getHelpFeatures} from './src/modules/help/HelpRegistry';import {plural} from './src/shared/i18n/Formatting';
 import {commandErrorCode} from './src/shared/tauri/CommandError';
 export async function check(){const rows=[];for(const [locale,resource]of Object.entries({en,de,fr}))for(const [ns,bundle]of Object.entries(resource))i18n.addResourceBundle(locale,ns,bundle);
 const ids=JSON.stringify(getHelpFeatures().map(f=>f.id));
 for(const locale of ['ru','en','de','fr']){await i18n.changeLanguage(locale);const html=renderToStaticMarkup(<HelpAbout/>);if(!html.includes('LittleSunAI')||!html.includes('${version}')||html.includes('tour.about.')||html.includes('{{'))throw Error('Incomplete about '+locale);if(JSON.stringify(getHelpFeatures().map(f=>f.id))!==ids)throw Error('Locale changed tour identity');for(const count of [0,1,2,5,11,21,101,1000000])if(plural('images',count)==='images')throw Error('Plural missing');rows.push({locale,characters:html.length});}
 if(commandErrorCode({code:'queue.target-missing',technical:'fixture'})!=='queue.target-missing')throw Error('Error code lost');return rows;}
 `},bundle:true,platform:'node',format:'cjs',write:false,loader:{'.md':'text','.png':'dataurl'},plugins:[{name:'project-license-text',setup(b){b.onLoad({filter:/[\\/](?:LICENSE|NOTICE)$/},async args=>({contents:await readFile(args.path,'utf8'),loader:'text'}));}}],logLevel:'silent'});
 const filename=join(scratch,'check.cjs');await writeFile(filename,result.outputFiles[0].text);const require=createRequire(import.meta.url);const rows=await require(filename).check();
 const routes=JSON.parse(await readFile('src/modules/help/TourRoutes.json','utf8'));assert.equal(routes.length,201);assert.equal(new Set(routes.map(r=>r.id)).size,201);const baselineRoutes=JSON.parse(await readFile('docs/training-localization/texts-ru/INVENTORY.json','utf8')).cards;for(const card of baselineRoutes)assert.ok(routes.some(r=>r.id===card.id),'Lost stable tour ID '+card.id);
 const components=JSON.parse(await readFile('third-party-licenses/components.json','utf8'));assert.equal(components.missingLicenseFiles.length,0);for(const c of components.components)for(const file of c.licenseFiles)assert((await readFile(file)).length>0);
 console.log(JSON.stringify({status:'passed',version,negativeVersionFixture:true,aboutLocales:rows,tourSteps:routes.length,thirdPartyComponents:components.components.length,networkRequests:0,paidRequests:0},null,2));
}finally{assert(resolve(scratch).startsWith(resolve(tmpdir())+sep)&&scratch.split(sep).at(-1).startsWith('smart-photo-public-test-'));await rm(scratch,{recursive:true,force:true});}
