export interface HelpRect {left:number;top:number;width:number;height:number}
/** Position from real card dimensions; avoid the highlighted control when space permits. */
export function helpTourLayout(target:HelpRect|null,card:HelpRect,viewport:{width:number;height:number}){
 const gap=12,w=Math.min(card.width,viewport.width-24),h=Math.min(card.height,viewport.height-24);
 const clamp=(v:number,size:number,limit:number)=>Math.max(gap,Math.min(v,limit-size-gap));
 const candidates=target?[
  {left:target.left+target.width+gap,top:target.top},
  {left:target.left-w-gap,top:target.top},
  {left:target.left,top:target.top+target.height+gap},
  {left:target.left,top:target.top-h-gap},
 ]:[{left:(viewport.width-w)/2,top:(viewport.height-h)/2}];
 const overlap=(p:{left:number;top:number})=>target?Math.max(0,Math.min(p.left+w,target.left+target.width)-Math.max(p.left,target.left))*Math.max(0,Math.min(p.top+h,target.top+target.height)-Math.max(p.top,target.top)):0;
 const placed=candidates.map(p=>({left:clamp(p.left,w,viewport.width),top:clamp(p.top,h,viewport.height)}));
 placed.sort((a,b)=>overlap(a)-overlap(b));const position=placed[0];
 let arrow:string|undefined;
 if(target&&overlap(position)===0){
  const tx=target.left+target.width/2,ty=target.top+target.height/2,cx=position.left+w/2,cy=position.top+h/2;
  const dx=tx-cx,dy=ty-cy,scale=1/Math.max(Math.abs(dx)/(w/2),Math.abs(dy)/(h/2));
  const endScale=1/Math.max(Math.abs(dx)/(target.width/2+4),Math.abs(dy)/(target.height/2+4));
  arrow=`M ${cx+dx*scale} ${cy+dy*scale} L ${tx-dx*endScale} ${ty-dy*endScale}`;
 }
 return {...position,arrow,maxHeight:viewport.height-position.top-gap};
}
