import {localeTag as displayLocaleTag} from "../../shared/i18n/Formatting";
import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import { useEffect, useMemo, useState } from 'react';
import { appLogService, APP_LOG_MAX_ENTRIES, type AppLogCategory, type AppLogEntry, type AppLogLevel, type AppLogWriteIntervalMs } from './AppLogService';
import '../../styles/app-log-panel.css';

interface LogPanelProps {
  entries: AppLogEntry[];
  onClose: () => void;
}

type LevelFilter = 'all' | AppLogLevel;
type CategoryFilter = 'all' | AppLogCategory;

const levelOptions: Array<{ value: LevelFilter; label: string }> = [
  { value: 'all', get label() { return localeText("main", "ui.LogPanel.dd00c2673bbfac54"); } },
  { value: 'error', get label() { return localeText("main", "ui.LogPanel.a6902f40c6d763c1"); } },
  { value: 'warning', get label() { return localeText("main", "ui.LogPanel.2f263a75997cc81f"); } },
  { value: 'info', get label() { return localeText("main", "ui.LogPanel.7134816372673a51"); } },
  { value: 'debug', get label() { return localeText("main", "ui.LogPanel.781eadc95932a335"); } },
];

const categoryOptions: Array<{ value: CategoryFilter; label: string }> = [
  { value: 'all', get label() { return localeText("main", "ui.LogPanel.029290156bbb8a58"); } },
  { value: 'local-ai', get label() { return localeText("main", "ui.LogPanel.921fef607cc1859e"); } },
  { value: 'runtime', get label() { return localeText("main", "ui.LogPanel.6071d29a377eb78d"); } },
  { value: 'network', get label() { return localeText("main", "ui.LogPanel.365cb827e0f46738"); } },
  { value: 'task', get label() { return localeText("main", "ui.LogPanel.6bbd047ebc064020"); } },
  { value: 'ui', get label() { return localeText("main", "ui.LogPanel.1d34780fff7158fc"); } },
  { value: 'app', get label() { return localeText("main", "ui.LogPanel.d1526272ac71a4e2"); } },
];

const intervalOptions: Array<{ value: AppLogWriteIntervalMs; label: string }> = [
  { value: 100, get label() { return localeText("main", "ui.LogPanel.cd50c307e4ebefb4"); } },
  { value: 500, get label() { return localeText("main", "ui.LogPanel.e1e47d9813ccfd59"); } },
  { value: 1000, get label() { return localeText("main", "ui.LogPanel.952d4a6614f513ad"); } },
  { value: 5000, get label() { return localeText("main", "ui.LogPanel.6f715faed4e838c3"); } },
];

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return date.toLocaleTimeString(displayLocaleTag(), { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function matchesSearch(entry: AppLogEntry, search: string): boolean {
  const clean = search.trim().toLowerCase();
  if (!clean) return true;
  return [entry.createdAt, entry.level, entry.category, entry.message, entry.details ?? '']
    .join('\n')
    .toLowerCase()
    .includes(clean);
}

export function LogPanel({ entries, onClose }: LogPanelProps) {
  useCurrentLocale();
  const [levelFilter, setLevelFilter] = useState<LevelFilter>('all');
  const [categoryFilter, setCategoryFilter] = useState<CategoryFilter>('all');
  const [writeIntervalMs, setWriteIntervalMs] = useState<AppLogWriteIntervalMs>(() => appLogService.getWriteIntervalMs());
  const [search, setSearch] = useState('');
  const [copyMessage, setCopyMessage] = useState('');

  useEffect(() => appLogService.subscribeWriteInterval(setWriteIntervalMs), []);

  const filteredEntries = useMemo(() => entries
    .filter((entry) => levelFilter === 'all' || entry.level === levelFilter)
    .filter((entry) => categoryFilter === 'all' || entry.category === categoryFilter)
    .filter((entry) => matchesSearch(entry, search))
    .slice()
    .reverse(), [entries, levelFilter, categoryFilter, search]);

  const copyFilteredLogs = async () => {
    const text = appLogService.exportText(filteredEntries.slice().reverse());
    try {
      await navigator.clipboard.writeText(text || localeText("main", "ui.LogPanel.2007b3c2217b627c"));
      setCopyMessage(localeText("main", "ui.LogPanel.98373b3702ebf8e2", {"value1": (filteredEntries.length)}));
    } catch(error) { setCopyMessage(localeText("main", "ui.LogPanel.1cf5de78c984d47b", {"value1": (String(error))})); }
  };

  return (
    <div className="log-panel-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="log-panel" role="dialog" aria-modal="true" aria-label={localeText("main", "ui.LogPanel.95fd7732bb8f5b0a")} onMouseDown={(event) => event.stopPropagation()}>
        <header className="log-panel__header">
          <div>
            <h2>{localeText("main", "ui.LogPanel.2dad5998c3156869")}</h2>
            <span>{filteredEntries.length} {' '}{localeText("main", "ui.LogPanel.6b21e9c97edfef53")}{' '}{entries.length} {' '}{localeText("main", "ui.LogPanel.7e2dc4b2f708a797")}{' '}{APP_LOG_MAX_ENTRIES}{localeText("main", "ui.LogPanel.76ce38eab5532e1e")}</span>
          </div>
          <Hint message={{id:"common.close-dialog"}}><button type="button" className="secondary-action" onClick={onClose}>{localeText("main", "ui.LogPanel.b45e57fa1333b644")}</button></Hint>
        </header>

        <div className="log-panel__filters">
          <Hint message={{id:"logs.level"}}><select value={levelFilter} onChange={(event) => setLevelFilter(event.target.value as LevelFilter)}>
            {levelOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
          </select></Hint>
          <Hint message={{id:"logs.category"}}><select value={categoryFilter} onChange={(event) => setCategoryFilter(event.target.value as CategoryFilter)}>
            {categoryOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
          </select></Hint>
          <Hint message={{id:"logs.interval"}}><select value={writeIntervalMs} onChange={(event) => appLogService.setWriteIntervalMs(Number(event.target.value) as AppLogWriteIntervalMs)}>
            {intervalOptions.map((option) => <option key={option.value} value={option.value}>{localeText("main", "ui.LogPanel.2d6df8d4265cebf7")}{' '}{option.label}</option>)}
          </select></Hint>
          <Hint message={{id:"logs.search"}}><input value={search} onChange={(event) => setSearch(event.target.value)} placeholder={localeText("main", "ui.LogPanel.5dd7cd4f994e002b")} /></Hint>
          <Hint message={{id:"logs.copy"}}><button type="button" className="secondary-action" onClick={() => void copyFilteredLogs()}>{localeText("main", "ui.LogPanel.7aa87e77d974081b")}</button></Hint>
          <Hint message={{id:"logs.clear"}}><button type="button" className="secondary-action danger-action" onClick={() => appLogService.clear()}>{localeText("main", "ui.LogPanel.96330e9e86de6565")}</button></Hint>
        </div>

        <div className="log-panel__list">
          {copyMessage ? <p role="status">{copyMessage}</p> : null}
          {filteredEntries.length > 0 ? filteredEntries.map((entry) => (
            <article key={entry.id} className={`log-entry log-entry--${entry.level}`}>
              <div className="log-entry__meta">
                <span>{formatTime(entry.createdAt)}</span>
                <strong>{entry.level}</strong>
                <em>{entry.category}</em>
              </div>
              <p>{entry.message}</p>
              {entry.details ? <pre>{entry.details}</pre> : null}
            </article>
          )) : (
            <div className="log-panel__empty">{localeText("main", "ui.LogPanel.b7ab375450da42c3")}</div>
          )}
        </div>
      </section>
    </div>
  );
}
