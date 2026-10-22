import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from 'react';

const widths = { table:0,list:260,small: 118, medium: 140, large: 158, extraLarge: 210 };
export function useVirtualGridItems<T extends { id: string }>(items: T[], viewSize: keyof typeof widths, focusedId: string | null,contextKey='default',suppressFocus?:{current:boolean}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const scrollPositions=useRef(new Map<string,number>()),restoredContext=useRef(''),skipFocus=useRef(false);
  const [geometry, setGeometry] = useState({ columns: 4, stride: 194, rowHeight: 176, gap: 18, height: 600, scroll: 0,headerHeight:0,paddingLeft:0,width:600 });
  const synchronizeScroll=(element:HTMLDivElement,position:number,origin:number)=>{
    element.scrollTop=position;
    const scroll=Math.max(0,element.scrollTop-origin);
    scrollPositions.current.set(contextKey,element.scrollTop);
    // Layout effects update the virtual window before paint, rather than waiting
    // for the next scroll event/rAF with new coordinates and stale cards.
    setGeometry(current=>current.scroll===scroll?current:{...current,scroll});
  };
  useLayoutEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    let frame = 0;
    const measure = () => {
      const css = getComputedStyle(element);
      const gap = parseFloat(css.columnGap) || 0;
      const rowHeight = parseFloat(css.gridAutoRows) || 176;
       const width = element.clientWidth - (parseFloat(css.paddingLeft) || 0) - (parseFloat(css.paddingRight) || 0);
       const headerHeight=(viewSize==='table'?36:0)+(parseFloat(css.paddingTop)||0);
       const next = { columns: viewSize==='table'?1:Math.max(1, Math.floor((width + gap) / ((parseFloat(css.getPropertyValue('--min-card-width')) || widths[viewSize]) + gap))),
         stride: rowHeight + (parseFloat(css.rowGap) || 0), rowHeight, gap, height: Math.max(0,element.clientHeight-headerHeight), scroll: Math.max(0,element.scrollTop-headerHeight),headerHeight,paddingLeft:parseFloat(css.paddingLeft)||0,width };
      setGeometry(current => Object.keys(next).every(key => current[key as keyof typeof next] === next[key as keyof typeof next]) ? current : next);
    };
    const schedule = () => { if (!frame) frame = requestAnimationFrame(() => { frame = 0; measure(); }); };
    measure();
    const observer = new ResizeObserver(schedule); observer.observe(element);
    const scrolled=()=>{if(restoredContext.current===contextKey){scrollPositions.current.set(contextKey,element.scrollTop);if(scrollPositions.current.size>32)scrollPositions.current.delete(scrollPositions.current.keys().next().value!);}schedule();};
    element.addEventListener('scroll', scrolled, { passive: true });
    return () => { observer.disconnect(); element.removeEventListener('scroll', scrolled); if (frame) cancelAnimationFrame(frame); };
  }, [items, viewSize,contextKey]);
  useLayoutEffect(()=>{
    const node=containerRef.current;if(!node||!items.length||restoredContext.current===contextKey)return;
    const position=scrollPositions.current.get(contextKey)??0;skipFocus.current=true;restoredContext.current=contextKey;
    synchronizeScroll(node,position,(viewSize==='table'?36:0)+(parseFloat(getComputedStyle(node).paddingTop)||0));
  },[contextKey,items.length]);
  const totalRows = Math.ceil(items.length / geometry.columns);
  const firstRow = Math.max(0, Math.floor(geometry.scroll / geometry.stride) - 3);
  const lastRow = Math.min(totalRows, Math.ceil((geometry.scroll + geometry.height) / geometry.stride) + 4);
  const start = Math.min(items.length, firstRow * geometry.columns);
  const end = Math.min(items.length, lastRow * geometry.columns);
  const visibleItems = useMemo(() => items.slice(start, end), [items, start, end]);
  const stickyHeight=viewSize==='table'?36:0;
  const previous=useRef<{items:T[];columns:number;stride:number;headerHeight:number;stickyHeight:number;contextKey:string}|null>(null);
  useLayoutEffect(()=>{
    const element=containerRef.current,old=previous.current;
    if(element&&old&&old.items.length&&old.contextKey===contextKey){
      const changed=old.items.some((item,index)=>items[index]?.id!==item.id);
      if(changed||old.columns!==geometry.columns||old.stride!==geometry.stride||old.headerHeight!==geometry.headerHeight){const scroll=Math.max(0,element.scrollTop+old.stickyHeight-old.headerHeight),row=Math.floor(scroll/old.stride),anchor=old.items[row*old.columns]?.id,index=items.findIndex(item=>item.id===anchor);
        if(index>=0 && element.scrollTop>0)synchronizeScroll(element,Math.max(0,geometry.headerHeight+Math.floor(index/geometry.columns)*geometry.stride+(scroll-row*old.stride)-stickyHeight),geometry.headerHeight);}
    }
    previous.current={items,columns:geometry.columns,stride:geometry.stride,headerHeight:geometry.headerHeight,stickyHeight,contextKey};
  },[items,geometry.columns,geometry.stride,geometry.headerHeight,stickyHeight,contextKey]);
  useLayoutEffect(() => {
    if(skipFocus.current){skipFocus.current=false;return;}
    if (suppressFocus?.current || !focusedId || !containerRef.current) return;
    const index = items.findIndex(item => item.id === focusedId);
    if (index < 0) return;
    const top = geometry.headerHeight+Math.floor(index / geometry.columns) * geometry.stride;
    const element = containerRef.current;
    if (top < element.scrollTop+stickyHeight) synchronizeScroll(element,Math.max(0,top-stickyHeight),geometry.headerHeight);
    else if (top + geometry.rowHeight > element.scrollTop + element.clientHeight) synchronizeScroll(element,Math.max(0, top + geometry.rowHeight - element.clientHeight),geometry.headerHeight);
    // Background metadata and scrolling must not bring the previous selection back into view.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [focusedId,contextKey]);
  const canvasStyle: CSSProperties = { position: 'relative', height: Math.max(0, totalRows * geometry.stride - (geometry.stride - geometry.rowHeight)) };
  const gridStyle: CSSProperties = { position: 'absolute', top: firstRow * geometry.stride, left: 0, right: 0,
    display: 'grid', gridTemplateColumns: `repeat(${geometry.columns}, minmax(0, 1fr))`, gridAutoRows: geometry.rowHeight,
    columnGap: geometry.gap, rowGap: geometry.stride - geometry.rowHeight };
  return { containerRef, visibleItems, canvasStyle, gridStyle,geometry };
}
