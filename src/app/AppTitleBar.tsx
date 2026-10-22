import { TitleBar } from './layout/TitleBar';

interface AppTitleBarProps {
  appName: string;
}

export function AppTitleBar({ appName }: AppTitleBarProps) {
  return <TitleBar appName={appName} />;
}
