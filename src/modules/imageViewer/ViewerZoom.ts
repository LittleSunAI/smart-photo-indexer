/** Fixed percentage points, with a single boundary at 100 and a finite manual range. */
export function nextViewerZoom(current:number,direction:1|-1):number{
 if(current<10)return direction===1?10:current;
 const step=direction===1?(current<100?5:25):(current<=100?5:25);
 // Fit values may be fractional. First manual step joins the same reversible lattice.
 const next=direction===1?(Math.floor(current/step+1e-8)+1)*step:(Math.ceil(current/step-1e-8)-1)*step;
 return Math.max(10,Math.min(800,next));
}

/** Wheel travel is additive in the old 5/25 point scale, independent of event splitting. */
export function wheelViewerZoom(current:number,pixels:number):number{
 if(!Number.isFinite(current)||!Number.isFinite(pixels)||!pixels)return current;
 if(current<10)return pixels<0?10:current;
 const position=current<=100?current/5:20+(current-100)/25;
 const next=position-1.18*pixels/120;
 return Math.max(10,Math.min(800,next<=20?next*5:100+(next-20)*25));
}
