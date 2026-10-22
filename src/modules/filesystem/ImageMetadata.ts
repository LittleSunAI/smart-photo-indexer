import {text as localeText} from "../../shared/i18n/Runtime";
import type { ImageFileMetadata } from '../explorer/types';
export function captureDateLabel(metadata?: ImageFileMetadata | null): string {
  if (!metadata?.dateTakenLocal) return localeText("main", "ui.ImageMetadata.82c387dd688b8ad5");
  const value=metadata.dateTakenLocal.replace('T',' ');
  return `${value} ${metadata.dateTakenOffset ?? localeText("main", "ui.ImageMetadata.9ab5e00f6a9b9eab")}`;
}
export function imageDimensions(metadata?: ImageFileMetadata | null): string {
  return metadata?.width && metadata.height ? `${metadata.width} × ${metadata.height}` : localeText("main", "ui.ImageMetadata.7d5dd2d26005862e");
}
