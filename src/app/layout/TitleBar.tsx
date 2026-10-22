import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import { messages } from '../../shared/i18n';

interface TitleBarProps {
  appName: string;
}



export function TitleBar({ appName }: TitleBarProps) {
  useCurrentLocale();
  return (
    <header className="title-bar">
      <nav className="title-bar__menu" aria-label={messages().titleBar.ariaMainMenu}>
        <div className="title-bar__app-icon" aria-hidden="true">
          {localeText("main", "ui.TitleBar.07a7a72a87848b88")}</div>

        {messages().titleBar.menuItems.map((item) => (
          <button key={item} className="title-bar__menu-button" type="button">
            {item}
          </button>
        ))}
      </nav>

      <div className="title-bar__app-name" title={appName}>
        {appName}
      </div>
    </header>
  );
}
