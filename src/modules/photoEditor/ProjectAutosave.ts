/** One deadline per dirty period. Updating text must not restart that deadline. */
export interface AutosaveState{owner:string;dirty:boolean;enabled:boolean;intervalMs:number;busy:boolean;savedToken:string|number|null}
export class ProjectAutosave{
 private state?:AutosaveState;private timer:ReturnType<typeof setTimeout>|undefined;private firstDirty:number|undefined;private lastSaved=0;private saving=false;private disposed=false;private retryAt=0;
 constructor(private save:()=>Promise<void>,private failed:(error:unknown)=>void,private clock=()=>Date.now()){}
 update(next:AutosaveState){if(this.disposed)return;const previous=this.state;if(previous?.owner!==next.owner){this.firstDirty=undefined;this.lastSaved=this.clock();this.retryAt=0;}else if(previous.savedToken!==next.savedToken){this.lastSaved=this.clock();this.firstDirty=next.dirty?this.lastSaved:undefined;this.retryAt=0;}
  this.state=next;if(next.dirty&&this.firstDirty===undefined)this.firstDirty=this.clock();if(!next.dirty)this.firstDirty=undefined;this.schedule();
 }
 private schedule(){if(this.timer!==undefined)clearTimeout(this.timer);this.timer=undefined;const s=this.state;if(this.disposed||this.saving||!s?.enabled||!s.dirty||this.firstDirty===undefined)return;const due=Math.max(this.firstDirty,this.lastSaved)+s.intervalMs;const delay=Math.max(0,due-this.clock(),this.retryAt-this.clock());this.timer=setTimeout(()=>void this.tick(),delay);}
 private async tick(){this.timer=undefined;const s=this.state;if(!s?.enabled||!s.dirty||this.disposed)return;if(s.busy){this.retryAt=this.clock()+1000;this.schedule();return;}this.saving=true;try{await this.save();this.lastSaved=this.clock();this.firstDirty=this.state?.dirty?this.lastSaved:undefined;this.retryAt=0;}catch(e){this.retryAt=this.clock()+5000;this.failed(e);}finally{this.saving=false;this.schedule();}}
 dispose(){this.disposed=true;if(this.timer!==undefined)clearTimeout(this.timer);}
}
