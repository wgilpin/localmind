import { vectorResults, recentNotes, retrievedDocuments } from './stores';
import type { VectorSearchResult } from './stores';

/**
 * Deletes a note and its associated vector entries from the database and updates stores.
 * @param noteId The ID of the note to delete.
 */
export async function deleteNote(noteId: string): Promise<void> {
    if (confirm('Are you sure you want to delete this note and its vector entries?')) {
        try {
            const response = await fetch(`/notes/${noteId}`, {
                method: 'DELETE',
            });
            if (response.ok) {
                // Update vectorResults store
                vectorResults.update(currentResults => currentResults.filter(note => note.id !== noteId));
                // Update recentNotes store
                recentNotes.update(current => ({
                    ...current,
                    notes: current.notes.filter(note => note.id !== noteId),
                }));
            } else {
                console.error('Failed to delete note:', response.statusText);
            }
        } catch (error) {
            console.error('Error deleting note:', error);
        }
    }
}

/**
 * Updates an existing note in the database and refreshes relevant stores.
 * @param noteId The ID of the note to update.
 * @param data The updated title and content of the note.
 */
export async function updateNote(noteId: string, data: { title: string, content: string }): Promise<void> {
    try {
        const response = await fetch(`/notes/${noteId}`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(data),
        });
        if (response.ok) {
            // Update vectorResults store
            // Update vectorResults store (only title, as VectorSearchResult does not have content)
            vectorResults.update(currentResults =>
                currentResults.map(note =>
                    note.id === noteId ? { ...note, title: data.title } : note
                )
            );
            // Update recentNotes store
            recentNotes.update(current => ({
                ...current,
                notes: current.notes.map(note =>
                    note.id === noteId ? { ...note, title: data.title, content: data.content } : note
                ),
            }));
            // Update retrievedDocuments store
            retrievedDocuments.update(currentDocs =>
                currentDocs.map(doc =>
                    doc.id === noteId ? { ...doc, title: data.title, content: data.content } : doc
                )
            );
        } else {
            console.error('Failed to update note:', response.statusText);
        }
    } catch (error) {
        console.error('Error updating note:', error);
    }
}