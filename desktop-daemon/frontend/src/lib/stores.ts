import { writable } from 'svelte/store';

export type VectorSearchResult = {
    id: string;
    title: string;
    url?: string;
    timestamp: number;
    chunk_text: string;
};

export type RetrievedDocument = {
    chunk_text: string;
};

export const searchResults = writable('');
export const vectorResults = writable<VectorSearchResult[]>([]);
export const retrievedDocuments = writable<RetrievedDocument[]>([]);
export const showResultsSection = writable(false);
export const showNewNoteSection = writable(false);

export type SearchStatus = 
  | 'idle'
  | 'starting'
  | 'embedding'
  | 'searching'
  | 'retrieving'
  | 'generating'
  | 'complete'
  | 'error';

export const searchStatus = writable<SearchStatus>('idle');
export const searchProgress = writable<string>('');

export const statusMessages: Record<SearchStatus, string> = {
  idle: '',
  starting: 'Starting search...',
  embedding: 'Processing query...',
  searching: 'Searching knowledge base...',
  retrieving: 'Retrieving relevant documents...',
  generating: 'Building response...',
  complete: 'Search complete',
  error: 'Search failed'
};