import type {GraphNode} from './HistoryGraph';
export type TreeMode='full'|'compact'|'cluster';
export interface TreeCamera {zoom:number;centerX:number;centerY:number}
export interface TreeRect {x:number;y:number;width:number;height:number}
export const TREE_METRICS={width:220,imageHeight:132,footer:32,compactFooter:6,clusterWidth:104,clusterHeight:46,overscan:180,maxZoom:3,minZoom:1e-7};
export function treeRect(node:Pick<GraphNode,'x'|'y'>,zoom:number,mode:TreeMode):TreeRect{
 return{x:node.x*zoom,y:node.y*zoom,width:mode==='cluster'?104:220*zoom,height:mode==='cluster'?46:132*zoom+(mode==='full'?32:6)};
}
export function rectsIntersect(a:TreeRect,b:TreeRect){return a.x+a.width>=b.x&&a.x<=b.x+b.width&&a.y+a.height>=b.y&&a.y<=b.y+b.height;}
export function rectBounds(rows:TreeRect[]):TreeRect{
 if(!rows.length)return{x:0,y:0,width:0,height:0};
 const x=Math.min(...rows.map(r=>r.x)),y=Math.min(...rows.map(r=>r.y));
 return{x,y,width:Math.max(...rows.map(r=>r.x+r.width))-x,height:Math.max(...rows.map(r=>r.y+r.height))-y};
}
export function cameraRect(camera:TreeCamera,size:{width:number;height:number},overscan=0):TreeRect{
 return{x:camera.centerX*camera.zoom-size.width/2-overscan,y:camera.centerY*camera.zoom-size.height/2-overscan,width:size.width+overscan*2,height:size.height+overscan*2};
}
/** Density is decided from graph coordinates, never the mounted/cull result. */
export function treeMode(nodes:GraphNode[],camera:TreeCamera,size:{width:number;height:number}):TreeMode{
 const mode=camera.zoom>=.75?'full':'compact';
 return camera.zoom<.2||nodes.filter(n=>rectsIntersect(treeRect(n,camera.zoom,mode),cameraRect(camera,size,180))).length>180?'cluster':mode;
}
export function treeClusters(nodes:GraphNode[],zoom:number,size:{width:number;height:number}){
 // Bound the worst-case grid occupancy, keeping grouping stable while panning.
 let pitch=120;while((Math.ceil((size.width+360)/pitch)+2)*(Math.ceil((size.height+360)/pitch)+2)>220)pitch*=1.5;
 const bins=new Map<string,{key:string;nodes:GraphNode[];rect:TreeRect}>(),nodeBins=new Map<string,string>();
 for(const node of nodes){const x=Math.floor(node.x*zoom/pitch)*pitch,y=Math.floor(node.y*zoom/pitch)*pitch,key=`${x}|${y}`;
  const bin=bins.get(key)??{key,nodes:[],rect:{x,y,width:104,height:46}};bin.nodes.push(node);bins.set(key,bin);nodeBins.set(node.step.id,key);
 }
 return{bins:[...bins.values()],nodeBins,pitch};
}
export function fitTree(nodes:GraphNode[],size:{width:number;height:number}):TreeCamera|null{
 if(!nodes.length||size.width<=0||size.height<=0)return null;
 const pad=Math.min(24,size.width/8,size.height/8),aw=size.width-2*pad,ah=size.height-2*pad;
 const minX=Math.min(...nodes.map(n=>n.x)),minY=Math.min(...nodes.map(n=>n.y)),dx=Math.max(...nodes.map(n=>n.x))-minX,dy=Math.max(...nodes.map(n=>n.y))-minY;
 let mode:TreeMode='full',zoom=Math.min(3,aw/(dx+220),(ah-32)/(dy+132));
 if(nodes.length>180){mode='cluster';}else if(zoom<.75){mode='compact';zoom=Math.min(.75-1e-9,aw/(dx+220),(ah-6)/(dy+132));if(zoom<.2)mode='cluster';}
 if(mode==='cluster'){
  const limit=(span:number,available:number,fixed:number)=>span>0?(available-fixed)/span:available>=fixed?3:TREE_METRICS.minZoom;
  zoom=Math.min(3,limit(dx,aw,104),limit(dy,ah,46),nodes.length>180?3:.2-1e-9);
 }
 zoom=Math.max(TREE_METRICS.minZoom,zoom);
 let bounds=rectBounds(mode==='cluster'?treeClusters(nodes,zoom,size).bins.map(b=>b.rect):nodes.map(n=>treeRect(n,zoom,mode)));
 for(let i=0;i<8&&(bounds.width>aw+.01||bounds.height>ah+.01);i++){
  zoom=Math.max(TREE_METRICS.minZoom,zoom*Math.min(aw/Math.max(aw,bounds.width),ah/Math.max(ah,bounds.height))*.995);
  mode=zoom<.2||nodes.length>180?'cluster':zoom<.75?'compact':'full';
  bounds=rectBounds(mode==='cluster'?treeClusters(nodes,zoom,size).bins.map(b=>b.rect):nodes.map(n=>treeRect(n,zoom,mode)));
 }
 return{zoom,centerX:(bounds.x+bounds.width/2)/zoom,centerY:(bounds.y+bounds.height/2)/zoom};
}
/** DOM scroll stays positive; the canonical world camera is allowed before the root. */
export function treeScrollAdapter(bounds:TreeRect,camera:TreeCamera,size:{width:number;height:number}){
 const visible=cameraRect(camera,size),mx=Math.max(96,size.width),my=Math.max(96,size.height);
 const originX=mx-Math.min(bounds.x,visible.x),originY=my-Math.min(bounds.y,visible.y);
 return{zoom:camera.zoom,originX,originY,width:Math.max(bounds.x+bounds.width,visible.x+visible.width)+originX+mx,height:Math.max(bounds.y+bounds.height,visible.y+visible.height)+originY+my,scrollLeft:camera.centerX*camera.zoom+originX-size.width/2,scrollTop:camera.centerY*camera.zoom+originY-size.height/2};
}
