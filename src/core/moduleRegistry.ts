import {text as localeText} from "../shared/i18n/Runtime";
import type { AppModuleDescriptor } from './moduleContracts';

export const appModules: AppModuleDescriptor[] = [
  { id: 'app-shell', get title() { return localeText("main", "ui.moduleRegistry.92bf2be379c869c7"); }, layer: 'ui', status: 'active', dependsOn: [] },
  { id: 'app-state', get title() { return localeText("main", "ui.moduleRegistry.3adbd445dff103be"); }, layer: 'state', status: 'active', dependsOn: ['app-shell'] },
  { id: 'explorer', get title() { return localeText("main", "ui.moduleRegistry.63c7847781879556"); }, layer: 'ui', status: 'stub', dependsOn: ['app-shell', 'app-state'] },
  { id: 'search', get title() { return localeText("main", "ui.moduleRegistry.560a49bcf443eb2d"); }, layer: 'ui', status: 'stub', dependsOn: ['index', 'app-state'] },
  { id: 'details', get title() { return localeText("main", "ui.moduleRegistry.e7ab7a7ea4d0163a"); }, layer: 'ui', status: 'stub', dependsOn: ['explorer', 'app-state'] },
  { id: 'index', get title() { return localeText("main", "ui.moduleRegistry.a0a12f0196bce98e"); }, layer: 'storage', status: 'planned', dependsOn: [] },
  { id: 'thumbnail', get title() { return localeText("main", "ui.moduleRegistry.d88eab5415fec12a"); }, layer: 'service', status: 'active', dependsOn: ['explorer'] },
  { id: 'task-queue', get title() { return localeText("main", "ui.moduleRegistry.6de2ff28b38fae9c"); }, layer: 'service', status: 'stub', dependsOn: ['app-state'] },
  { id: 'ai', get title() { return localeText("main", "ui.moduleRegistry.25ee7667872a7ec1"); }, layer: 'integration', status: 'stub', dependsOn: ['task-queue', 'index'] },
  { id: 'rename', get title() { return localeText("main", "ui.moduleRegistry.4adde04fda009a22"); }, layer: 'service', status: 'planned', dependsOn: ['index', 'action-log'] },
  { id: 'action-log', get title() { return localeText("main", "ui.moduleRegistry.62d5a1e3c4ac6a2c"); }, layer: 'storage', status: 'stub', dependsOn: ['index'] },
  { id: 'settings', get title() { return localeText("main", "ui.moduleRegistry.ec0708c09eb27d4b"); }, layer: 'state', status: 'stub', dependsOn: ['app-state'] },
];
