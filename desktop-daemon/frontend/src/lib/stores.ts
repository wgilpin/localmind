import { writable } from 'svelte/store';

export const searchResults = writable('');
export const showResultsSection = writable(false);
export const showNewNoteSection = writable(false);