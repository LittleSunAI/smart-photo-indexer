import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect, useState } from 'react';
import {invoke} from "../../shared/tauri/invoke";
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ViewerWindow } from './ViewerWindow';
import type { ViewerSessionPayload } from './viewerSession';

export function ViewerBootstrap() {
  useCurrentLocale();
  const label = getCurrentWindow().label;
  const [session, setSession] = useState<ViewerSessionPayload | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = () => invoke<ViewerSessionPayload>('get_viewer_session', { label });
  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    void listen<ViewerSessionPayload>('smart-photo-indexer:viewer-native-session', event => {
      if (!cancelled) setSession(current => (event.payload.navigationRevision ?? 0) >= (current?.navigationRevision ?? 0) ? event.payload : current);
    }, { target: label }).then(cleanup => {
      if (cancelled) cleanup(); else unlisten = cleanup;
      return load();
    }).then(value => { if (!cancelled) setSession(current => (value.navigationRevision ?? 0) >= (current?.navigationRevision ?? 0) ? value : current); })
      .catch(reason => { if (!cancelled) setError(String(reason)); });
    return () => { cancelled = true; unlisten?.(); };
  }, [label]);
  if (session) return <ViewerWindow session={session} />;
  if(!error)return <div className="smart-image-viewer-window" aria-busy="true"/>;
  return <div className="smart-image-viewer-window">
    <div className="smart-image-viewer__fallback" style={{ minHeight: '100vh' }}>
      <strong>{error ? localeText("main", "ui.ViewerBootstrap.433abb7a50239838") : localeText("main", "ui.ViewerBootstrap.0f0310da3eba1fcb")}</strong>
      {error ? <span role="alert">{error}</span> : null}
      {error ? <Hint message={{id:"viewer.retry-session"}}><button type="button" onClick={() => void load().then(setSession).catch(reason => setError(String(reason)))}>{localeText("main", "ui.ViewerBootstrap.e854774074d0e642")}</button></Hint> : null}
      <Hint message={{id:"viewer.close"}}><button type="button" onClick={() => void getCurrentWindow().close()}>{localeText("main", "ui.ViewerBootstrap.486c1a14895b9fad")}</button></Hint>
    </div>
  </div>;
}
