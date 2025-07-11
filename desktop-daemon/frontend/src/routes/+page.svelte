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
    display: grid;
    grid-template-rows: auto auto 1fr; /* Header, Search, and then flexible content area */
    height: 100vh;
    overflow: hidden; /* Prevent overall scrollbar */
  }

  .header {
    background-color: var(--color-background);
    padding: 1rem;
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.1);
    display: flex;
    align-items: center;
    justify-content: space-between;
    z-index: 100; /* Ensure header is above scrolling content */
  }

  .search-section {
    padding: 1rem;
    background-color: var(--color-background); /* Ensure search has a background */
    z-index: 90; /* Below header */
  }

  .content-area {
    overflow-y: auto; /* Only this area will scroll */
    padding: 1rem; /* Add padding to the scrollable area */
  }

  h1 {
    margin: 0;
  }

  h2 {
    margin-top: 0; /* Adjust heading margin for better spacing */
  }

  .settings-button {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--color-text);
  }
</style>

<div id="container">
  <header class="header">
    <h1>LocalMind</h1>
    <button class="settings-button" on:click={() => showSettingsSection.set(true)}>
      ⚙️
    </button>
  </header>

  <section class="search-section">
    <h2>Search</h2>
    <Search />
  </section>

  <main class="content-area">
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
  </main>
</div>
<FAB />