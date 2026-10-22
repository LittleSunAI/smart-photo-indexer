import { AppShell } from './AppShell';
import { AppStateProvider } from './state/AppStateContext';

export function App() {
  return (
    <AppStateProvider>
      <AppShell />
    </AppStateProvider>
  );
}
