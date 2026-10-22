import {text as localeText, useCurrentLocale} from "../../shared/i18n/Runtime";
import {Hint} from "../../shared/help/Hint";
import {Icon} from '../../shared/ui/Icon';
import { useMemo, useState, useRef, type ChangeEvent, type MouseEvent } from 'react';
import {useDialogFocus} from '../../shared/ui/useDialogFocus';
import type {
  SmartImageResizeFormat,
  SmartImageResizeMode,
  SmartImageViewerChooseSavePathInput,
  SmartImageViewerFileActions,
  SmartImageViewerItem,
  SmartImageViewerSaveResizeInput,
} from './ImageViewer.types';

interface ImageViewerResizeDialogProps {
  item: SmartImageViewerItem;
  fileActions: SmartImageViewerFileActions;
  onClose(): void;
  onSaved?(targetPath: string): void;
}

interface ParsedDimensions {
  width: number;
  height: number;
}

const outputFormats: SmartImageResizeFormat[] = ['jpeg', 'png', 'tiff', 'bmp'];

export function ImageViewerResizeDialog({ item, fileActions, onClose, onSaved }: ImageViewerResizeDialogProps) {
  useCurrentLocale();
  const parsedDimensions = useMemo(() => parseDimensions(item.dimensions ?? ''), [item.dimensions]);
  const original = parsedDimensions ?? {width:0,height:0};
  const [mode, setMode] = useState<SmartImageResizeMode>('pixels');
  const [width, setWidth] = useState(original.width);
  const [height, setHeight] = useState(original.height);
  const [percent, setPercent] = useState(100);
  const [quality, setQuality] = useState(90);
  const [format, setFormat] = useState<SmartImageResizeFormat>(guessInitialFormat(item.extension));
  const [message, setMessage] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const dialog=useRef<HTMLElement>(null);useDialogFocus(dialog,onClose,isSaving);

  const currentSummary = `${parsedDimensions ? `${original.width} × ${original.height}` : localeText("viewer", "ui.ImageViewerResizeDialog.9deb7219b25adac7")} · ${item.sizeLabel ?? formatBytes(item.sizeBytes)} · ${(item.extension ?? 'image').toUpperCase()}`;
  const nextSummary = localeText("viewer", "ui.ImageViewerResizeDialog.677748dac2cf6872", {"value1": (width), "value2": (height), "value3": (formatBytes(estimateNewSize(item.sizeBytes, original, { width, height }, quality, format))), "value4": (format.toUpperCase())});
  const canSave = Boolean(parsedDimensions&&width>0&&height>0&&fileActions.chooseSaveImagePath && fileActions.saveResizedImage);

  const updatePixelsWidth = (nextWidth: number) => {
    const cleanWidth = clampInteger(nextWidth, 1, 50000);
    setWidth(cleanWidth);
    setHeight(Math.max(1, Math.round(cleanWidth * original.height / original.width)));
    setPercent(Math.max(1, Math.round(cleanWidth / original.width * 100)));
  };

  const updatePixelsHeight = (nextHeight: number) => {
    const cleanHeight = clampInteger(nextHeight, 1, 50000);
    setHeight(cleanHeight);
    setWidth(Math.max(1, Math.round(cleanHeight * original.width / original.height)));
    setPercent(Math.max(1, Math.round(cleanHeight / original.height * 100)));
  };

  const updatePercent = (nextPercent: number) => {
    const cleanPercent = clampInteger(nextPercent, 1, 2000);
    setPercent(cleanPercent);
    setWidth(Math.max(1, Math.round(original.width * cleanPercent / 100)));
    setHeight(Math.max(1, Math.round(original.height * cleanPercent / 100)));
  };

  const handleSave = async () => {
    if (isSaving || !canSave) return;
    if (!fileActions.chooseSaveImagePath || !fileActions.saveResizedImage) {
      setMessage(localeText("viewer", "ui.ImageViewerResizeDialog.8119e62941986223"));
      return;
    }

    setIsSaving(true);
    setMessage(null);

    try {
      const chooseInput: SmartImageViewerChooseSavePathInput = {
        sourcePath: item.path,
        defaultFileName: buildDefaultResizeName(item.name, format),
        format,
      };
      const chosen = await fileActions.chooseSaveImagePath(chooseInput);
      if (!chosen.selectedPath) {
        setMessage(localeText("viewer", "ui.ImageViewerResizeDialog.d4d67fd162b648fa"));
        return;
      }

      const saveInput: SmartImageViewerSaveResizeInput = {
        sourcePath: item.path,
        targetPath: chosen.selectedPath,
        width,
        height,
        quality,
        format,
      };
      const result = await fileActions.saveResizedImage(saveInput);
      setMessage(localeText("viewer", "ui.ImageViewerResizeDialog.c6486a9d8a7da63b", {"value1": (result.fileName)}));
      onSaved?.(result.targetPath);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="smart-image-viewer-resize-backdrop" role="presentation" onMouseDown={(event: MouseEvent<HTMLDivElement>) => event.stopPropagation()}>
      <section ref={dialog} tabIndex={-1} className="smart-image-viewer-resize-dialog" role="dialog" aria-modal="true" aria-label={localeText("viewer", "ui.ImageViewerResizeDialog.123501882b32a350")}>
        <header className="smart-image-viewer-resize-dialog__header">
          <h2>{localeText("viewer", "ui.ImageViewerResizeDialog.c6569839f006bd96")}</h2>
          <Hint message={{id:"common.close-dialog"}}><button type="button" onClick={onClose} disabled={isSaving} aria-label={localeText("viewer", "ui.ImageViewerResizeDialog.277e92039e84be98")}><Icon name="close"/></button></Hint>
        </header>

        {!parsedDimensions ? (
          <p className="smart-image-viewer-resize-dialog__warning">
            {localeText("viewer", "ui.ImageViewerResizeDialog.5d41dd8238f577ae")}</p>
        ) : null}

        {parsedDimensions ? <><div className="smart-image-viewer-resize-dialog__mode-row">
          <Hint message={{id:"resize.pixels"}}><label>
            <input
              type="radio"
              disabled={isSaving}
              checked={mode === 'pixels'}
              onChange={() => {
                setMode('pixels');
                updatePixelsWidth(width);
              }}
            />
            {localeText("viewer", "ui.ImageViewerResizeDialog.cba42f31593eb4f0")}</label></Hint>
          <Hint message={{id:"resize.percent"}}><label>
            <input
              type="radio"
              disabled={isSaving}
              checked={mode === 'percent'}
              onChange={() => {
                setMode('percent');
                updatePercent(percent);
              }}
            />
            {localeText("viewer", "ui.ImageViewerResizeDialog.24373d0649f38001")}</label></Hint>
        </div>

        {mode === 'pixels' ? (
          <div className="smart-image-viewer-resize-dialog__field-grid">
            <Hint message={{id:"resize.width"}}><label>
              <span>{localeText("viewer", "ui.ImageViewerResizeDialog.301e353fa933e39e")}</span>
              <input type="number" disabled={isSaving} min={1} max={50000} value={width} onChange={(event: ChangeEvent<HTMLInputElement>) => updatePixelsWidth(Number(event.target.value))} />
            </label></Hint>
            <Hint message={{id:"resize.height"}}><label>
              <span>{localeText("viewer", "ui.ImageViewerResizeDialog.5419f94a577e19d9")}</span>
              <input type="number" disabled={isSaving} min={1} max={50000} value={height} onChange={(event: ChangeEvent<HTMLInputElement>) => updatePixelsHeight(Number(event.target.value))} />
            </label></Hint>
          </div>
        ) : (
          <div className="smart-image-viewer-resize-dialog__field-grid">
            <Hint message={{id:"resize.percent-value"}}><label>
              <span>{localeText("viewer", "ui.ImageViewerResizeDialog.d85119ba1aebae3c")}</span>
              <input type="number" disabled={isSaving} min={1} max={2000} value={percent} onChange={(event: ChangeEvent<HTMLInputElement>) => updatePercent(Number(event.target.value))} />
            </label></Hint>
            <Hint message={{id:"resize.percent-value"}}><label>
              <span>{localeText("viewer", "ui.ImageViewerResizeDialog.fc10621ba039dc76")}</span>
              <input type="number" disabled={isSaving} min={1} max={2000} value={percent} onChange={(event: ChangeEvent<HTMLInputElement>) => updatePercent(Number(event.target.value))} />
            </label></Hint>
          </div>
        )}

        <p className="smart-image-viewer-resize-dialog__ratio">{localeText("viewer", "ui.ImageViewerResizeDialog.0e7d5de09687986f")}</p>

        {format==='jpeg'?<Hint message={{id:"resize.quality"}}><label className="smart-image-viewer-resize-dialog__quality">
          <span>{localeText("viewer", "ui.ImageViewerResizeDialog.9c86b17fe6d3266d")}{' '}{quality}</span>
          <input type="range" disabled={isSaving} min={1} max={100} value={quality} onChange={(event: ChangeEvent<HTMLInputElement>) => setQuality(clampInteger(Number(event.target.value), 1, 100))} />
        </label></Hint>:null}

        <Hint message={{id:"resize.format"}}><label className="smart-image-viewer-resize-dialog__format">
          <span>{localeText("viewer", "ui.ImageViewerResizeDialog.a2ce873274bdea7a")}</span>
          <select disabled={isSaving} value={format} onChange={(event: ChangeEvent<HTMLSelectElement>) => setFormat(event.target.value as SmartImageResizeFormat)}>
            {outputFormats.map((candidate) => (
              <option key={candidate} value={candidate}>{candidate.toUpperCase()}</option>
            ))}
          </select>
        </label></Hint>
        </> : null}

        <div className="smart-image-viewer-resize-dialog__summary">
          <strong>{localeText("viewer", "ui.ImageViewerResizeDialog.d25e69c48a62d5a5")}</strong>
          <span>{currentSummary}</span>
          {parsedDimensions ? <><strong>{localeText("viewer", "ui.ImageViewerResizeDialog.c48d576c03a93f64")}</strong>
          <span>{nextSummary}</span></> : null}
        </div>

        {message ? <p className="smart-image-viewer-resize-dialog__message">{message}</p> : null}

        <footer className="smart-image-viewer-resize-dialog__footer">
          <Hint message={{id:parsedDimensions?'resize.save':'resize.unavailable'}}><button type="button" onClick={handleSave} disabled={isSaving || !canSave} title={!parsedDimensions ? localeText("viewer", "ui.ImageViewerResizeDialog.c6e21d8cd71dc5b4") : !canSave ? localeText("viewer", "ui.ImageViewerResizeDialog.649e5a865ba927ae") : undefined}>
            {isSaving ? localeText("viewer", "ui.ImageViewerResizeDialog.8b13719b54cbbfac") : localeText("viewer", "ui.ImageViewerResizeDialog.d51e6f10cb84e96c")}
          </button></Hint>
          <Hint message={{id:"common.cancel-draft"}}><button type="button" disabled={isSaving} onClick={onClose}>{localeText("viewer", "ui.ImageViewerResizeDialog.f6731ec05bd0562d")}</button></Hint>
        </footer>
      </section>
    </div>
  );
}

export function parseDimensions(value: string): ParsedDimensions | null {
  const match = value.match(/(\d+)\s*[×xX]\s*(\d+)/);
  if (!match) return null;

  const width = Number(match[1]);
  const height = Number(match[2]);
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) return null;

  return { width, height };
}

function clampInteger(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

function guessInitialFormat(extension?: string): SmartImageResizeFormat {
  const clean = extension?.trim().toLowerCase().replace(/^\./, '');
  if (clean === 'png') return 'png';
  if (clean === 'tif' || clean === 'tiff') return 'tiff';
  if (clean === 'bmp') return 'bmp';
  return 'jpeg';
}

function buildDefaultResizeName(fileName: string, format: SmartImageResizeFormat): string {
  const extension = format === 'jpeg' ? 'jpg' : format === 'tiff' ? 'tiff' : format;
  const cleanName = fileName.trim() || 'image';
  const base = cleanName.replace(/\.[^.]+$/, '');
  return `${base}-resized.${extension}`;
}

function estimateNewSize(
  sourceSizeBytes: number | undefined,
  original: ParsedDimensions,
  next: ParsedDimensions,
  quality: number,
  format: SmartImageResizeFormat,
): number | undefined {
  if (!sourceSizeBytes || sourceSizeBytes <= 0) return undefined;
  const originalPixels = Math.max(1, original.width * original.height);
  const nextPixels = Math.max(1, next.width * next.height);
  const pixelRatio = nextPixels / originalPixels;
  const qualityRatio = format === 'jpeg' ? Math.max(0.1, quality / 90) : 1;
  return Math.max(1, Math.round(sourceSizeBytes * pixelRatio * qualityRatio));
}

function formatBytes(value?: number): string {
  if (!value || !Number.isFinite(value) || value < 0) return localeText("viewer", "ui.ImageViewerResizeDialog.961cd69981ec0037");
  if (value < 1024) return localeText("viewer", "ui.ImageViewerResizeDialog.0ed91d812c4b0aec", {"value1": (value)});
  if (value < 1024 * 1024) return localeText("viewer", "ui.ImageViewerResizeDialog.4ff1063b0138f8b5", {"value1": ((value / 1024).toFixed(1))});
  if (value < 1024 * 1024 * 1024) return localeText("viewer", "ui.ImageViewerResizeDialog.ae30f202fd93338e", {"value1": ((value / 1024 / 1024).toFixed(1))});
  return localeText("viewer", "ui.ImageViewerResizeDialog.f8a8d68f96d764b3", {"value1": ((value / 1024 / 1024 / 1024).toFixed(1))});
}
