import { useEffect, useMemo, useState } from 'react';
import type {
  SmartImageViewerFileActions,
  SmartImageViewerItem,
  SmartImageViewerWindowMode,
} from './ImageViewer.types';
import { SmartImageViewer } from './SmartImageViewer';
import { subscribeSmartImageViewerOpen } from './imageViewerBus';
import { tauriSmartImageViewerFileActions } from './TauriSmartImageViewerFileService';

interface SmartImageViewerHostProps<TImage extends SmartImageViewerItem = SmartImageViewerItem> {
  images: TImage[];
  selectedImageId?: string | null;
  fileActions?: SmartImageViewerFileActions;
  onToggleFavorite(itemId: string): void | Promise<void>;
  onViewerOpen?(itemId: string): void;
  onViewerClose?(): void;
}

export function SmartImageViewerHost<TImage extends SmartImageViewerItem = SmartImageViewerItem>({
  images,
  selectedImageId = null,
  fileActions = tauriSmartImageViewerFileActions,
  onToggleFavorite,
  onViewerOpen,
  onViewerClose,
}: SmartImageViewerHostProps<TImage>) {
  const [isOpen, setIsOpen] = useState(false);
  const [itemId, setItemId] = useState<string | null>(null);
  const [windowMode, setWindowMode] = useState<SmartImageViewerWindowMode>('full');
  const [detailsOpen, setDetailsOpen] = useState(false);

  useEffect(() => {
    return subscribeSmartImageViewerOpen((nextItemId) => {
      setItemId(nextItemId);
      setIsOpen(true);
      setWindowMode((currentMode) => currentMode === 'minimized' ? 'full' : currentMode);
      onViewerOpen?.(nextItemId);
    });
  }, [onViewerOpen]);

  const item = useMemo(() => {
    const targetId = itemId ?? selectedImageId;
    if (!targetId) return null;
    return images.find((candidate) => candidate.id === targetId) ?? null;
  }, [images, itemId, selectedImageId]);

  const handleOpenImage = (nextItemId: string) => {
    setItemId(nextItemId);
    setIsOpen(true);
    onViewerOpen?.(nextItemId);
  };

  const handleClose = () => {
    setIsOpen(false);
    setItemId(null);
    setDetailsOpen(false);
    onViewerClose?.();
  };

  if (!isOpen || !item) return null;

  return (
    <SmartImageViewer
      item={item}
      allImages={images}
      windowMode={windowMode}
      detailsOpen={detailsOpen}
      fileActions={fileActions}
      onClose={handleClose}
      onWindowModeChange={setWindowMode}
      onToggleDetails={() => setDetailsOpen((value) => !value)}
      onToggleFavorite={onToggleFavorite}
      onOpenImage={handleOpenImage}
    />
  );
}
