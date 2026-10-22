export interface SearchQuery {
  text: string;
  includeTags: boolean;
  includeDescription: boolean;
  includeFileName: boolean;
}

export interface SearchResult {
  imageId: string;
  score: number;
  matchedFields: string[];
}
