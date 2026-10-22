import type{EditorStep}from'./EditorContracts';
export const GRAPH_CARD={width:220,height:164,column:268,row:196,padding:24};
export interface GraphNode{step:EditorStep;level:number;row:number;x:number;y:number;invalid:boolean}
export interface HistoryGraph{nodes:GraphNode[];byId:Map<string,GraphNode>;children:Map<string,string[]>;parents:Map<string,string[]>;edges:{parent:string;child:string}[];width:number;height:number;invalidCount:number}
const order=(a:EditorStep,b:EditorStep)=>a.createdAtUnix-b.createdAtUnix||a.id.localeCompare(b.id,'en');
export function historyGraph(steps:EditorStep[]):HistoryGraph{
 const unique=new Map<string,EditorStep>();for(const step of steps)if(!unique.has(step.id))unique.set(step.id,step);
 const children=new Map<string,string[]>(),parents=new Map<string,string[]>(),pending=new Map<string,number>(),levels=new Map<string,number>(),invalid=new Set<string>();
 for(const s of unique.values()){const ps=[...new Set(s.parentIds)].filter(id=>{if(!unique.has(id)){invalid.add(s.id);return false;}return true;});parents.set(s.id,ps);pending.set(s.id,ps.length);for(const p of ps){const rows=children.get(p)??[];rows.push(s.id);children.set(p,rows);}}
 const queue=[...unique.values()].filter(s=>pending.get(s.id)===0).sort(order);for(const s of queue)levels.set(s.id,0);
 for(let i=0;i<queue.length;i++){const node=queue[i];for(const id of children.get(node.id)??[]){levels.set(id,Math.max(levels.get(id)??0,(levels.get(node.id)??0)+1));pending.set(id,pending.get(id)!-1);if(pending.get(id)===0)queue.push(unique.get(id)!);}}
 for(const id of unique.keys())if(pending.get(id)!>0){levels.set(id,0);invalid.add(id);}
 const columns=new Map<number,EditorStep[]>();for(const s of unique.values()){const level=levels.get(s.id)??0;const col=columns.get(level)??[];col.push(s);columns.set(level,col);}
 const nodes:GraphNode[]=[],byId=new Map<string,GraphNode>();
 for(const[level,column]of [...columns].sort(([a],[b])=>a-b)){column.sort((a,b)=>{const group=(s:EditorStep)=>Math.min(...(parents.get(s.id)??[]).map(p=>byId.get(p)?.row??0),0x7fffffff);return group(a)-group(b)||order(a,b);});column.forEach((step,row)=>{const node={step,level,row,x:level*GRAPH_CARD.column,y:row*GRAPH_CARD.row,invalid:invalid.has(step.id)};nodes.push(node);byId.set(step.id,node);});}
 const edges=[];for(const node of nodes)for(const parent of parents.get(node.step.id)??[])if(byId.get(parent)!.level<node.level)edges.push({parent,child:node.step.id});
 return{nodes,byId,children,parents,edges,width:Math.max(GRAPH_CARD.width,...nodes.map(n=>n.x+GRAPH_CARD.width)),height:Math.max(GRAPH_CARD.height,...nodes.map(n=>n.y+GRAPH_CARD.height)),invalidCount:invalid.size};
}
export function graphAncestors(graph:HistoryGraph,id:string){const result=new Set<string>(),queue=[id];for(let i=0;i<queue.length;i++){if(result.has(queue[i]))continue;result.add(queue[i]);queue.push(...(graph.parents.get(queue[i])??[]));}return result;}
export function graphVisible(graph:HistoryGraph,collapsed:Set<string>){const visible=new Set<string>(),queue=graph.nodes.filter(n=>!(graph.parents.get(n.step.id)?.length)||n.invalid).map(n=>n.step.id);for(let i=0;i<queue.length;i++){const id=queue[i];if(visible.has(id))continue;visible.add(id);if(!collapsed.has(id))queue.push(...(graph.children.get(id)??[]));}return visible;}
