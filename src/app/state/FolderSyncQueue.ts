import type { ExplorerItem } from '../../modules/explorer/types';

// One native writer, at most four pages retained. Enumeration is independent
// of expensive hashing until this bounded backlog is full.
export function createFolderSyncQueue(sync:(items:ExplorerItem[])=>Promise<void>,signal:AbortSignal){
  let tail=Promise.resolve();
  const outstanding=new Set<Promise<void>>();
  let failure:unknown;
  return {
    async enqueue(items:ExplorerItem[]){
      signal.throwIfAborted();if(failure)throw failure;
      const next=tail.then(async()=>{signal.throwIfAborted();await sync(items);});
      tail=next.catch(error=>{failure=error;});
      outstanding.add(next);void next.then(()=>outstanding.delete(next),()=>outstanding.delete(next));
      if(outstanding.size>=4)await Promise.race(outstanding);
    },
    async drain(){await tail;signal.throwIfAborted();if(failure)throw failure;},
  };
}
