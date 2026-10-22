import {displayNumber} from '../i18n/Formatting';
import {localeTag as displayLocaleTag} from "../i18n/Formatting";
import {text as localeText} from "../i18n/Runtime";
export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes)) {
    return localeText("common", "ui.fileFormatting.91eed1160fbbf8eb");
  }

  if (bytes < 1024) {
    return `${displayNumber(bytes)} B`;
  }

  const units = ['KB', 'MB', 'GB', 'TB'];
  let value = bytes / 1024;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const digits = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${displayNumber(value,{minimumFractionDigits:digits,maximumFractionDigits:digits})} ${units[unitIndex]}`;
}

export function formatUnixTime(seconds: number | null | undefined): string {
  if (!seconds) {
    return localeText("common", "ui.fileFormatting.1c53c68a197bf172");
  }

  try {
    return new Intl.DateTimeFormat(displayLocaleTag(), {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(seconds * 1000));
  } catch {
    return String(seconds);
  }
}

export function extensionToFileType(extension: string | null | undefined): string {
  if (!extension) {
    return 'File';
  }

  return extension.replace(/^\./, '').toUpperCase();
}
