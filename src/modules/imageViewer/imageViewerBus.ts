type SmartImageViewerOpenListener = (itemId: string) => void;

const openListeners = new Set<SmartImageViewerOpenListener>();

export function notifySmartImageViewerOpen(itemId: string): void {
  const cleanItemId = itemId.trim();
  if (!cleanItemId) return;

  for (const listener of openListeners) {
    listener(cleanItemId);
  }
}

export function subscribeSmartImageViewerOpen(listener: SmartImageViewerOpenListener): () => void {
  openListeners.add(listener);
  return () => {
    openListeners.delete(listener);
  };
}
