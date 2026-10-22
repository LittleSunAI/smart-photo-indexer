import { useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent } from 'react';
import { createPortal } from 'react-dom';
import { Icon } from './Icon';
import {Hint} from '../help/Hint';
import type {HelpMessage} from '../help/HelpMessage';

export interface SelectOption { value: string; label: string; title?:string }
export function CompactSelect({ caption, value, options, onChange, className = '',helpId,tourId,icon,message,supplement,disabled=false }: {
  caption: string; value: string; options: SelectOption[]; onChange(value: string): void; className?: string;helpId?:string;tourId?:string;icon?:Parameters<typeof Icon>[0]['name'];message?:HelpMessage;supplement?:HelpMessage;disabled?:boolean;
}) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  useEffect(() => { if (disabled) setOpen(false); }, [disabled]);
  const [active, setActive] = useState(0);
  const [position, setPosition] = useState({ left: 0, top: 0, width: 180, height: 280 });
  const selected = Math.max(0, options.findIndex(option => option.value === value));
  const optionLabels=options.map(option=>option.label).join('\u0000');
  const close = (focus = false) => { setOpen(false); if (focus) trigger.current?.focus(); };
  const show = () => { if (!disabled) { setActive(selected); setOpen(true); } };
  useLayoutEffect(() => {
    if (!open) return;
    const place = () => {
      const rect = trigger.current?.getBoundingClientRect(); if (!rect) return;
      const rowsHeight=options.length*32+10;
      const wanted=Math.min(280,rowsHeight),below=window.innerHeight-rect.bottom-12,above=rect.top-12,down=below>=wanted||below>=above;
      const height=Math.max(32,Math.min(wanted,down?below:above));
      const measure=document.createElement('canvas').getContext('2d'),font=getComputedStyle(menu.current??trigger.current!);if(measure)measure.font=`${font.fontWeight} 13px ${font.fontFamily}`;
      const needed=(measure?Math.max(...options.map(o=>measure.measureText(o.label).width)):80)+46+(height<rowsHeight?16:0);
      const width=Math.min(Math.max(rect.width,needed),window.innerWidth-16);
      setPosition({ left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)),
        top: down ? rect.bottom + 4 : Math.max(8, rect.top - height - 4), width, height });
    };
    let active=true;place();void document.fonts?.ready.then(()=>{if(active)place();});window.addEventListener('resize', place); window.addEventListener('scroll', place, true);
    return () => { active=false;window.removeEventListener('resize', place); window.removeEventListener('scroll', place, true); };
  }, [open, optionLabels]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (!trigger.current?.contains(event.target as Node) && !menu.current?.contains(event.target as Node)) close();
    };
    document.addEventListener('pointerdown', outside);
    return () => document.removeEventListener('pointerdown', outside);
  }, [open]);
  useEffect(() => { if (open) menu.current?.querySelector(`[data-option-index="${active}"]`)?.scrollIntoView({ block: 'nearest' }); }, [active, open]);
  const choose = (index: number) => { const option = options[index]; if (option) onChange(option.value); close(true); };
  const key = (event: KeyboardEvent) => {
    if (event.key === 'Tab') { close(); return; }
    if (event.key === 'Escape' && open) { event.preventDefault(); close(true); return; }
    if (['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter', ' '].includes(event.key)) {
      event.preventDefault();
      if (!open) { show(); return; }
      if (event.key === 'Enter' || event.key === ' ') choose(active);
      else setActive(index => event.key === 'Home' ? 0 : event.key === 'End' ? options.length - 1 :
        Math.max(0, Math.min(options.length - 1, index + (event.key === 'ArrowDown' ? 1 : -1))));
    }
  };
  const control=<button ref={trigger} disabled={disabled} data-help-id={helpId} data-tour-id={tourId} type="button" className={`compact-select ${className}`} role="combobox" aria-label={caption}
      aria-haspopup="listbox" aria-expanded={open} aria-controls={open ? id : undefined}
      aria-activedescendant={open ? `${id}-${active}` : undefined} onKeyDown={key} onClick={() => open ? close() : show()}>
      <span className="compact-select__compact-icon"><Icon name={icon??(helpId==='sort'?'sortOrder':value==='table'?'viewTable':'viewGrid')}/></span><span className="compact-select__caption">{caption}</span><span className="compact-select__value" title={options[selected]?.title??options[selected]?.label}>{options[selected]?.label}</span><Icon name="down" />
    </button>;
  return <>
    {message?<Hint message={message} supplement={supplement} popupOpen={open}>{control}</Hint>:control}
    {open ? createPortal(<div ref={menu} id={id} role="listbox" aria-label={caption} className="compact-select__menu"
      style={{ left: position.left, top: position.top, width: position.width, maxHeight: position.height }}>
      {options.map((option, index) => <div key={option.value} id={`${id}-${index}`} role="option" aria-selected={option.value === value}
        data-option-index={index} className={active === index ? 'is-active' : ''} title={option.title??option.label}
        onMouseEnter={() => setActive(index)} onMouseDown={event => event.preventDefault()} onClick={() => choose(index)}>
        <span>{option.label}</span>{option.value === value ? <Icon name="check" /> : null}
      </div>)}
    </div>, document.body) : null}
  </>;
}
