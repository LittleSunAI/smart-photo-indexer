import type { ExplorerItem, ImageItem, MockPhotoTheme, ProcessingStatus } from './types';

function createImage(index: number, status: ProcessingStatus, theme: MockPhotoTheme): ImageItem {
  const padded = String(index).padStart(4, '0');
  const name = `IMG_2024_0421_${padded}.jpg`;

  return {
    id: `image-${padded}`,
    kind: 'image',
    name,
    path: `D:\\Photos\\2024\\Birthday\\${name}`,
    status,
    dateTaken: '21.04.2024 19:42:17',
    sizeLabel: '5.2 MB',
    dimensions: '5472 × 3648',
    tags: ['birthday', 'cake', 'party', 'family', 'celebration', 'candles', 'friends', 'people'],
    description:
      'A family birthday celebration indoors. People are gathered around a table with a cake and warm decorative lights in the background.',
    theme,
  };
}

export function createMockExplorerItems(): ExplorerItem[] {
  return [
    { id: 'folder-parent', kind: 'folder', name: '..', path: 'D:\\Photos\\2024', childCount: 0 },
    { id: 'folder-family', kind: 'folder', name: 'Family', path: 'D:\\Photos\\2024\\Birthday\\Family', childCount: 128 },
    { id: 'folder-friends', kind: 'folder', name: 'Friends', path: 'D:\\Photos\\2024\\Birthday\\Friends', childCount: 84 },
    { id: 'folder-pets', kind: 'folder', name: 'Pets', path: 'D:\\Photos\\2024\\Birthday\\Pets', childCount: 56 },
    createImage(1, 'processed', 'birthday'),
    createImage(2, 'processed', 'balloons'),
    createImage(3, 'processed', 'outdoorParty'),
    createImage(4, 'processed', 'familyTable'),
    createImage(5, 'processed', 'dog'),
    createImage(6, 'notProcessed', 'mountains'),
    createImage(7, 'processed', 'sunset'),
    createImage(8, 'processed', 'dinner'),
    createImage(9, 'queued', 'kidsCake'),
    createImage(10, 'processed', 'girlsBalloons'),
    createImage(11, 'queued', 'cat'),
    createImage(12, 'processed', 'forest'),
    createImage(13, 'notProcessed', 'hikers'),
    createImage(14, 'error', 'city'),
    createImage(15, 'processed', 'picnic'),
    createImage(16, 'notProcessed', 'pier'),
    createImage(17, 'processed', 'familyPortrait'),
  ];
}
