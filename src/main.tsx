import {startLocaleController} from './shared/i18n/LocaleController';
import {useCurrentLocale} from './shared/i18n/Runtime';
import {text as localeText} from "./shared/i18n/Runtime";
import ReactDOM from 'react-dom/client';
import { lazy, Suspense } from 'react';
import './styles/global.css';
import './styles/explorer-views.css';
import './styles/ui-refinement.css';
import './styles/toolbar-layout.css';
import './styles/processing-toolbar.css';
import { isTauriRuntime } from './shared/tauri/runtime';
import {startThemeController} from './shared/ThemeController';
import {HelpHost} from './modules/help/HelpHost';
import {HintHost} from './shared/help/HintHost';
startThemeController();

const App = lazy(() => import('./app/App').then(module => ({ default: module.App })));
const ViewerBootstrap = lazy(() => import('./modules/imageViewerWindow/ViewerBootstrap').then(module => ({ default: module.ViewerBootstrap })));
const EditorBootstrap = lazy(() => import('./modules/photoEditor/EditorBootstrap').then(module => ({default:module.EditorBootstrap})));

function shouldRenderViewerWindow(): boolean {
  if (!isTauriRuntime() || typeof window === 'undefined') return false;

  const params = new URLSearchParams(window.location.search);
  return params.get('viewer') === '1' || params.get('window') === 'viewer';
}

function WindowRoot(){
 useCurrentLocale();
 return (
  <><Suspense fallback={<div role="status">{localeText("main", "ui.main.73cc8333f9bc2df9")}</div>}>
    {isTauriRuntime()&&new URLSearchParams(window.location.search).get('window')==='editor'?<EditorBootstrap/>:shouldRenderViewerWindow() ? <ViewerBootstrap /> : <App />}
  </Suspense><HelpHost/><HintHost/></>
);
}
void startLocaleController().then(()=>{
 ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(<WindowRoot/>);
});
