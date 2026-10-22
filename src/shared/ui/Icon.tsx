import type {CSSProperties} from 'react';

const names=['computer','filter','help','sun','moon','edit','rotateLeft','rotateRight','mirrorHorizontal','mirrorVertical','trash','print','share','more','minimize','maximize','restore','close','star','info','play','pause','fit','plus','minus','fullscreen','left','right','down','up','check','folder','copy','clipboard','eye','eyeOff','refresh','stop','settings','image','collection','search','file','undo','redo','link','bulletList','numberedList','tag','editorSelect','editorPan','editorBrush','editorEraser','opInstruction','opReplaceObject','opRemoveObject','opRemoveBackground','opReplaceBackground','opExpand','opEnhance','opLighting','opRecolor','opCompose','opRestore','opStartAgain','viewGrid','viewTable','folderOrder','imageFit','sortOrder'] as const;
const assets=import.meta.glob<string>(['../../assets/icons/*.png','../../assets/icons/*.svg','!../../assets/icons/*sheet*.png'],{eager:true,query:'?url',import:'default'});
export function Icon({name,filled=false}:{name:typeof names[number];filled?:boolean}){
  const vector=['mirrorHorizontal','mirrorVertical','restore'].includes(name)?assets['../../assets/icons/'+name+'.svg']:undefined;
  const asset=vector??assets['../../assets/icons/'+(filled&&name==='star'?'starFilled':name)+'.png']??assets['../../assets/icons/'+name+'.svg'];
  const style:CSSProperties={maskImage:'url("'+asset+'")',WebkitMaskImage:'url("'+asset+'")'};
  return <span className="control-icon" style={style} aria-hidden="true"/>;
}
