import { writable } from 'svelte/store';

export type VectorSearchResult = {
    id: string;
    title: string;
    url?: string;
    timestamp: number;
    chunk_text: string;
};

export type RetrievedDocument = {
    id: string; // Added id property
    title: string;
    content: string;
};

export const searchResults = writable('');
export const vectorResults = writable<VectorSearchResult[]>([]);
export const retrievedDocuments = writable<RetrievedDocument[]>([]);
export const showResultsSection = writable(false);
export const showNewNoteSection = writable(false);
export const showSettingsSection = writable(false);

export type Document = {
    id: string;
    content: string;
    url?: string;
    title: string;
    timestamp: number;
};

export const recentNotes = writable<{
    notes: Document[];
    page: number;
    hasMore: boolean;
}>({ notes: [], page: 0, hasMore: true });

export type SearchStatus = 
  | 'idle'
  | 'starting'
  | 'embedding'
  | 'searching'
  | 'retrieving'
  | 'generating'
  | 'complete'
  | 'error'
  | 'stopped'; // Added 'stopped' status

export const searchStatus = writable<SearchStatus>('idle');
export const searchProgress = writable<string>('');

// Store for the EventSource instance
export const currentEventSource = writable<EventSource | null>(null);

export const statusMessages: Record<SearchStatus, string> = {
  idle: '',
  starting: 'Starting search...',
  embedding: 'Processing query...',
  searching: 'Searching knowledge base...',
  retrieving: 'Retrieving relevant documents...',
  generating: 'Building response...',
  complete: 'Search complete',
  error: 'Search failed',
  stopped: 'Search stopped by user' // Message for 'stopped' status
};

/**
 * Sends a request to the backend to stop any ongoing generation.
 */
export async function stopCurrentGeneration() {
  if (currentEventSource) {
    currentEventSource.update(es => {
      if (es) {
        es.close();
      }
      return null;
    });
  }
  try {
    const response = await fetch('/stop-generation', { method: 'POST' });
    if (!response.ok) {
      console.error('Failed to stop generation on backend.');
    }
    searchStatus.set('stopped');
    searchProgress.set(statusMessages.stopped);
  } catch (error) {
    console.error('Error sending stop signal:', error);
  }
}