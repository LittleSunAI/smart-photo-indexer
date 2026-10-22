export type SlideshowPhase='view'|'entering'|'show'|'exiting';
/** One serialized native transition; the latest requested mode wins. */
export class SlideshowTransition{
 phase:SlideshowPhase='view';desired=false;
 private active=false;private pending?:Promise<void>;private disposed=false;
 constructor(private native:(enabled:boolean)=>Promise<void>,private changed:(phase:SlideshowPhase)=>void){}
 private publish(phase:SlideshowPhase){this.phase=phase;if(!this.disposed)this.changed(phase);}
 request(enabled:boolean):Promise<void>{
  if(this.disposed)return Promise.resolve();this.desired=enabled;
  return this.pending??(this.pending=Promise.resolve().then(()=>this.run()).finally(()=>{this.pending=undefined;}));
 }
 private async run(){
  while(!this.disposed&&this.active!==this.desired){
   const next=this.desired;this.publish(next?'entering':'exiting');
   try{await this.native(next);}catch(error){this.desired=this.active;this.publish(this.active?'show':'view');throw error;}
   this.active=next;
   if(this.disposed){if(next)await this.native(false).catch(()=>{});this.active=false;return;}
   this.publish(next?'show':'view');
  }
 }
 dispose(){this.disposed=true;this.desired=false;if(!this.pending&&this.active){this.active=false;void this.native(false).catch(()=>{});}}
}
