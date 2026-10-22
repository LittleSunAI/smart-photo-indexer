import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {Icon} from '../../shared/ui/Icon';
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type RefObject } from 'react';
import {createPortal} from 'react-dom';
import {useDialogFocus} from '../../shared/ui/useDialogFocus';
import type { AppErrorIssue } from './AppError';
import { messages } from '../../shared/i18n';

interface ErrorCenterProps {
  issues: AppErrorIssue[];
  onDismiss(issueId: string): void;
  onClearAll(): void;
}

function formatIssueTime(unix: number): string {
  return new Date(unix * 1000).toLocaleTimeString(displayLocaleTag(), { hour: '2-digit', minute: '2-digit' });
}

export function ErrorCenter({ issues, onDismiss, onClearAll }: ErrorCenterProps) {
  const returnFocus=useRef<HTMLElement|null>(null);
  useEffect(()=>{
    const remember=(event:FocusEvent)=>{const target=event.target;if(target instanceof HTMLElement&&target!==document.body&&!target.closest('.error-center'))returnFocus.current=target;};
    document.addEventListener('focusin',remember);
    return()=>document.removeEventListener('focusin',remember);
  },[]);
  const visibleIssues = useMemo(() => issues.filter((issue) => !issue.isDismissed), [issues]);
  return visibleIssues.length?<ErrorDialog issues={visibleIssues} onDismiss={onDismiss} onClearAll={onClearAll} returnFocus={returnFocus}/>:null;
}

function ErrorDialog({issues:visibleIssues,onDismiss,onClearAll,returnFocus}:ErrorCenterProps&{returnFocus:RefObject<HTMLElement|null>}){
  useCurrentLocale();
  const [expanded,setExpanded]=useState(false),dialog=useRef<HTMLDialogElement>(null),messageId=useId();
  const latest=visibleIssues[0];
  useLayoutEffect(()=>{
    const element=dialog.current,active=document.activeElement,previous=active instanceof HTMLElement&&active!==document.body?active:returnFocus.current;
    element?.showModal();
    return()=>{element?.close();if(previous?.isConnected)previous.focus();};
  },[returnFocus]);
  useDialogFocus(dialog,onClearAll);

  return createPortal(
    <dialog ref={dialog} className={`error-center error-center--${latest.severity}`} role="alertdialog" aria-label={messages().errors.aria} aria-describedby={messageId} onCancel={event=>{event.preventDefault();onClearAll();}}>
      <div className="error-center__summary">
        <span className="error-center__icon" aria-hidden="true">{latest.severity === 'critical' || latest.severity === 'error' ? '⚠' : '!'}</span>
        <div className="error-center__text">
          <strong>{latest.title}</strong>
          <span id={messageId}>{latest.userMessage}</span>
          {latest.relatedPath ? <small title={latest.relatedPath}>{latest.relatedPath}</small> : null}
        </div>
        <Hint message={{id:"errors.expand"}}><button type="button" className="secondary-action" onClick={() => setExpanded((value) => !value)}>
          {expanded ? messages().errors.hideDetails : messages().errors.issueCount(visibleIssues.length)}
        </button></Hint>
        <Hint message={{id:"common.close-dialog"}}><button type="button" className="icon-button" autoFocus aria-label={messages().errors.closeDialog} onClick={onClearAll}><Icon name="close"/></button></Hint>
      </div>

      {expanded ? (
        <div className="error-center__details">
          <div className="error-center__toolbar">
            <span>{messages().errors.keepHere}</span>
            <Hint message={{id:"errors.dismiss-all"}}><button type="button" className="secondary-action" onClick={onClearAll}>{messages().errors.dismissAll}</button></Hint>
          </div>
          <ul className="error-center__list">
            {visibleIssues.slice(0, 6).map((issue) => (
              <li key={issue.id}>
                <header>
                  <strong>{issue.title}</strong>
                  <span>{messages().errors.domains[issue.domain]}</span>
                  <time>{formatIssueTime(issue.createdAtUnix)}</time>
                  {issue.retryable ? <em>{messages().errors.retryable}</em> : null}
                </header>
                <p>{issue.userMessage}</p>
                {issue.operation ? <small>{messages().errors.operation}: {issue.operation}</small> : null}
                <details>
                  <Hint message={{id:"common.expand",args:{раздел:localeText("errors", "ui.ErrorCenter.2d3b84d814755c39")}}}><summary>{messages().errors.technicalDetails}</summary></Hint>
                  <code>{issue.technicalMessage}</code>
                  <ul>
                    {issue.suggestions.map((suggestion) => <li key={suggestion}>{suggestion}</li>)}
                  </ul>
                </details>
                <Hint message={{id:"errors.dismiss"}}><button type="button" className="secondary-action" onClick={() => onDismiss(issue.id)}>{messages().errors.dismiss}</button></Hint>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </dialog>,document.body
  );
}
