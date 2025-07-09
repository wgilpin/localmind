<script lang="ts">
  // @ts-nocheck
  import Search, { showResultsSection } from '$lib/components/Search.svelte';
  import Results from '$lib/components/Results.svelte';
  import NewNote, { showNewNoteSection } from '$lib/components/NewNote.svelte';
  import FAB from '$lib/components/FAB.svelte';
</script>

<div id="container">
    <h1>LocalMind</h1>

    <section>
        <h2>Search</h2>
        <Search />
    </section>

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