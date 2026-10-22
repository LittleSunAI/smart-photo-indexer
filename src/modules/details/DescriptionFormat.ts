export interface DescriptionRun {text:string;bold?:boolean;italic?:boolean;underline?:boolean;link?:string}
export interface DescriptionBlock {kind:'paragraph'|'bullet'|'number';runs:DescriptionRun[]}
export interface DescriptionFormat {version:1;blocks:DescriptionBlock[]}
export const descriptionText=(format:DescriptionFormat)=>format.blocks.map(block=>block.runs.map(run=>run.text).join('')).join('\n').trim();
export const safeDescriptionLink=(value:string)=>{try{const url=new URL(value);return ['http:','https:'].includes(url.protocol)&&!url.username&&!url.password&&value.length<=2048&&!/[\x00-\x1f\x7f]/.test(value)?url.href:null;}catch{return null;}};

// Convert browser editing nodes to a small document; pasted HTML is never retained.
export function readDescriptionDocument(root:HTMLElement):DescriptionFormat{
 const blocks:DescriptionBlock[]=[];let block:DescriptionBlock={kind:'paragraph',runs:[]};
 const flush=(force=false)=>{if(force||block.runs.length){blocks.push(block);block={kind:'paragraph',runs:[]};}};
 const visit=(node:Node,style:Omit<DescriptionRun,'text'>={})=>{
  if(node.nodeType===Node.TEXT_NODE){const parts=(node.textContent??'').replace(/\r\n?/g,'\n').split('\n');parts.forEach((text,index)=>{if(index)flush(true);if(text)block.runs.push({text,...style});});return;}
  if(!(node instanceof HTMLElement))return;
  const tag=node.tagName.toLowerCase();if(['script','style','img','iframe','object','video','audio'].includes(tag))return;
  if(tag==='br'){flush(true);return;}
  const container=['div','p','li','ul','ol'].includes(tag);
  if(container)flush();
  if(tag==='li')block.kind=node.parentElement?.tagName==='OL'?'number':'bullet';
  const next={...style};if(['b','strong'].includes(tag)||Number(node.style.fontWeight)>=600)next.bold=true;
  if(['i','em'].includes(tag)||node.style.fontStyle==='italic')next.italic=true;
  if(tag==='u'||node.style.textDecoration.includes('underline'))next.underline=true;
  if(tag==='a'){const link=safeDescriptionLink(node.getAttribute('href')??'');if(link)next.link=link;}
  node.childNodes.forEach(child=>visit(child,next));if(container)flush(tag==='p'||tag==='li'||tag==='div');
 };
 root.childNodes.forEach(node=>visit(node));flush();
 // Empty paragraphs at the end of browser list containers do not add description text.
 while(blocks.length>1&&!blocks.at(-1)!.runs.length)blocks.pop();
 return {version:1,blocks};
}
export function renderDescriptionDocument(root:HTMLElement,text:string,format?:DescriptionFormat|null){
 root.replaceChildren();
 const valid=format?.version===1&&descriptionText(format)===text.trim();
 const blocks:DescriptionBlock[]=valid?format!.blocks:text.split('\n').map(text=>({kind:'paragraph' as const,runs:[{text}]}));
 let list:HTMLElement|null=null,kind='';
 for(const block of blocks){
  let parent=root;
  if(block.kind!=='paragraph'){
   if(kind!==block.kind||!list){list=document.createElement(block.kind==='bullet'?'ul':'ol');root.append(list);kind=block.kind;}
   parent=list;
  }else{list=null;kind='';}
  const line=document.createElement(block.kind==='paragraph'?'div':'li');
  for(const run of block.runs){let node:Node=document.createTextNode(run.text);
   for(const [enabled,tag] of [[run.bold,'strong'],[run.italic,'em'],[run.underline,'u']] as const){if(enabled){const wrap=document.createElement(tag);wrap.append(node);node=wrap;}}
   const link=run.link&&safeDescriptionLink(run.link);if(link){const anchor=document.createElement('a');anchor.href=link;anchor.append(node);node=anchor;}
   line.append(node);
  }
  if(!line.childNodes.length)line.append(document.createElement('br'));parent.append(line);
 }
}
