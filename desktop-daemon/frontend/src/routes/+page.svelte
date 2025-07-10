<script lang="ts">
  // @ts-nocheck
  import Search from '$lib/components/Search.svelte';
  import Results from '$lib/components/Results.svelte';
  import NewNote from '$lib/components/NewNote.svelte';
  import FAB from '$lib/components/FAB.svelte';
  import { showResultsSection, showNewNoteSection, showSettingsSection } from '$lib/stores';
  import Settings from '$lib/components/Settings.svelte';
</script>

<div id="container">
    <h1>LocalMind</h1>
    <button class="settings-button" on:click={() => showSettingsSection.set(true)}>
        ⚙️
    </button>

    <section>
        <h2>Search</h2>
        <Search />
    </section>

    {#if $showSettingsSection}
        <section id="settings-section">
            <h2>Settings</h2>
            <Settings />
        </section>
    {/if}

    {#if $showResultsSection}
        <section id="results-section">
            <h2>Results</h2>
            <Results />
        </section>
    {/if}

    {#if $showNewNoteSection}
        <section id="new-note-section">
            <h2>New Note</h2>
            <NewNote />
        </section>
    {/if}
</div>
<FAB />