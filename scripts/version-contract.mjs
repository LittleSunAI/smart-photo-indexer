import fs from 'node:fs';
import path from 'node:path';
export function verifyVersions(root){
 const json=file=>JSON.parse(fs.readFileSync(path.join(root,file),'utf8'));
 const pkg=json('package.json'),lock=json('package-lock.json'),config=json('src-tauri/tauri.conf.json');
 const toml=fs.readFileSync(path.join(root,'src-tauri/Cargo.toml'),'utf8');
 const section=toml.match(/^\[package\]\s*([\s\S]*?)(?=^\[|$(?![\s\S]))/m)?.[1]??'';
 const cargo=section.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
 const lockText=fs.readFileSync(path.join(root,'src-tauri/Cargo.lock'),'utf8');
 const own=lockText.split('[[package]]').find(block=>/^name\s*=\s*"smart-photo-indexer"/m.test(block));
 const cargoLock=own?.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
 const values={'package.json':pkg.version,'package-lock.json':lock.version,'package-lock root':lock.packages?.['']?.version,'tauri.conf.json':config.version,'Cargo.toml':cargo,'Cargo.lock':cargoLock};
 if(!/^\d+\.\d+\.\d+(?:-[\w.-]+)?$/.test(pkg.version??''))throw Error('Invalid package version');
 for(const [file,value]of Object.entries(values))if(value!==pkg.version)throw Error(`Version mismatch: ${file}=${value}; package.json=${pkg.version}`);
 return pkg.version;
}
