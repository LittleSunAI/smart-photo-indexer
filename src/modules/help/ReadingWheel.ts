/** Local reading surface: preserve fractional travel and ignore our rounded scroll acknowledgements. */
export function attachReadingWheel(owner:HTMLElement){
 let logical=owner.scrollTop,lastWritten=owner.scrollTop;
 const externalScroll=()=>{if(owner.scrollTop!==lastWritten){logical=owner.scrollTop;lastWritten=owner.scrollTop;}};
 const wheel=(event:WheelEvent)=>{
  if(event.ctrlKey||event.shiftKey||!event.deltaY)return;
  const target=event.target instanceof Element?event.target:null;
  if(target?.closest('input,select,textarea,[contenteditable="true"]'))return;
  // A nested scroller keeps its own wheel; the article does not process the same event twice.
  for(let node=target;node&&node!==owner;node=node.parentElement){
   const style=getComputedStyle(node);
   if(/auto|scroll/.test(style.overflowY)&&node.scrollHeight>node.clientHeight)return;
  }
  event.preventDefault();event.stopPropagation();externalScroll();
  const pixels=event.deltaY*(event.deltaMode===1?16:event.deltaMode===2?owner.clientHeight:1);
  if(!Number.isFinite(pixels))return;
  const maximum=Math.max(0,owner.scrollHeight-owner.clientHeight);
  logical=Math.max(0,Math.min(maximum,logical+pixels*.4));
  owner.scrollTop=logical;lastWritten=owner.scrollTop;
 };
 owner.addEventListener('wheel',wheel,{passive:false});owner.addEventListener('scroll',externalScroll);
 return()=>{owner.removeEventListener('wheel',wheel);owner.removeEventListener('scroll',externalScroll);};
}
