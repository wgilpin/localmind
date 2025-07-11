<script lang="ts">
  // @ts-nocheck
  import Search from '$lib/components/Search.svelte';
  import Results from '$lib/components/Results.svelte';
  import NewNote from '$lib/components/NewNote.svelte';
  import FAB from '$lib/components/FAB.svelte';
  import RecentNotes from '$lib/components/RecentNotes.svelte';
  import { showResultsSection, showNewNoteSection, showSettingsSection, searchStatus } from '$lib/stores';
  import Settings from '$lib/components/Settings.svelte';
</script>

<style>
  #container {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .header {
    position: sticky;
    top: 0;
    z-index: 100;
    background-color: var(--color-background); /* Ensure it has a background */
    padding: 1rem;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .content-area {
    flex-grow: 1;
    overflow-y: auto;
  }

  h1 {
    margin: 0;
  }

  .settings-button {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--color-text);
  }

  section {
    padding: 1rem;
  }
</style>

<div id="container">
  <div class="header">
    <h1>LocalMind</h1>
    <button class="settings-button" on:click={() => showSettingsSection.set(true)}>
      ⚙️
    </button>
  </div>

  <section>
    <h2>Search</h2>
    <Search />
  </section>

  <div class="content-area">
    {#if $showSettingsSection}
      <section id="settings-section">
        <h2>Settings</h2>
        <Settings />
      </section>
    {:else if $showResultsSection}
      <section id="results-section">
        <h2>Results</h2>
        <Results />
      </section>
    {:else if $showNewNoteSection}
      <section id="new-note-section">
        <h2>New Note</h2>
        <NewNote />
      </section>
    {:else if $searchStatus === 'idle' || $searchStatus === 'stopped'}
      <section id="recent-notes-section">
        <h2>Recent Notes</h2>
        <RecentNotes />
      </section>
    {/if}
  </div>
</div>
<FAB />