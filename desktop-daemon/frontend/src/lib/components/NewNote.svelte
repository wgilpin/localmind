<script lang="ts">
  import { writable } from 'svelte/store';

  export const showNewNoteSection = writable(false);

  let noteTitle = '';
  let noteContent = '';

  async function saveNote() {
    if (!noteTitle || !noteContent) {
      alert('Please enter a title and content for the note.');
      return;
    }

    try {
      const response = await fetch('/documents', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          title: noteTitle,
          content: noteContent,
          url: `note://${Date.now()}`
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to save note');
      }

      noteTitle = '';
      noteContent = '';
      showNewNoteSection.set(false);
      alert('Note saved successfully!');

    } catch (error) {
      console.error('Error saving note:', error);
      alert('Failed to save note. See console for details.');
    }
  }
</script>

<div class="space-y-2">
    <input type="text" id="note-title" placeholder="Note Title" class="input" bind:value={noteTitle}>
    <textarea id="note-content" placeholder="Note Content" class="textarea" bind:value={noteContent}></textarea>
    <button id="save-note-button" class="btn" on:click={saveNote}>Save Note</button>
</div>