export interface MarqueeGeometry {columns:number;stride:number;rowHeight:number;gap:number;headerHeight:number;paddingLeft:number;width:number}
export interface ContentRect {left:number;top:number;right:number;bottom:number}
/** Hit test the same regular layout that the virtual grid renders, including unmounted rows. */
export function marqueeIds<T extends {id:string}>(items:T[],g:MarqueeGeometry,r:ContentRect):string[]{
 const result:string[]=[],cardWidth=(g.width-g.gap*(g.columns-1))/g.columns,columnStride=cardWidth+g.gap;
 const first=Math.max(0,Math.ceil((r.top-g.headerHeight-g.rowHeight)/g.stride)),last=Math.min(Math.ceil(items.length/g.columns)-1,Math.floor((r.bottom-g.headerHeight)/g.stride));
 const startColumn=Math.max(0,Math.ceil((r.left-g.paddingLeft-cardWidth)/columnStride)),lastColumn=Math.min(g.columns-1,Math.floor((r.right-g.paddingLeft)/columnStride));
 for(let row=first;row<=last;row++)for(let column=startColumn;column<=lastColumn;column++){const item=items[row*g.columns+column];if(item)result.push(item.id);}
 return result;
}
