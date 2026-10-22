export interface PhotoCollection {
  id: string;
  name: string;
  description?: string | null;
  color?: string | null;
  isFavorite: boolean;
  imageCount: number;
  createdAtUnix: number;
  updatedAtUnix: number;
}

export interface CollectionsResult {
  collections: PhotoCollection[];
  totalCount: number;
  loadedAtUnix: number;
}

export interface CreateCollectionInput {
  name: string;
  description?: string | null;
  color?: string | null;
}

export interface AddImagesToCollectionInput {
  collectionId: string;
  imageIds: string[];
}

export interface CollectionMutationResult {
  collection?: PhotoCollection | null;
  requestedImages: number;
  changedImages: number;
  skippedImages: number;
  actionLogEntries: number;
  updatedAtUnix: number;
}

export interface SetImageFavoriteInput {
  imageId?: string | null;
  path?: string | null;
  isFavorite: boolean;
}

export interface SetImageFavoriteResult {
  imageId: string;
  path: string;
  fileName: string;
  isFavorite: boolean;
  updatedAtUnix: number;
  actionLogId: number;
}

export interface CollectionsState {
  isLoading: boolean;
  errorMessage: string | null;
  items: PhotoCollection[];
  lastLoadedAtUnix: number | null;
  lastMutationMessage: string | null;
}
