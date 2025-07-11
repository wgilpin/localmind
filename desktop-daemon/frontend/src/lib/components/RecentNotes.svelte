<script lang="ts">
  import { onMount } from 'svelte';
  import { recentNotes } from '$lib/stores';
  import Documents from './Documents.svelte'; // Import the Documents component

  let loading = false;
  let observer: IntersectionObserver;
  let endOfResults: HTMLElement;

  async function fetchRecentNotes() {
    if (loading || !$recentNotes.hasMore) {
      return;
    }

    loading = true;
    const limit = 10;
    const offset = $recentNotes.page * limit;

    try {
      const response = await fetch(`/recent-notes?limit=${limit}&offset=${offset}`);
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const newNotes = await response.json();

      recentNotes.update(current => ({
        notes: [...current.notes, ...newNotes],
        page: current.page + 1,
        hasMore: newNotes.length === limit,
      }));
    } catch (error) {
      console.error('Error fetching recent notes:', error);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchRecentNotes();

    observer = new IntersectionObserver((entries) => {
      if (entries[0].isIntersecting && !loading && $recentNotes.hasMore) {
        fetchRecentNotes();
      }
    }, {
      rootMargin: '100px',
    });

    if (endOfResults) {
      observer.observe(endOfResults);
    }

    return () => {
      if (observer) {
        observer.disconnect();
      }
    };
  });
</script>

<style>
  .recent-notes-container {
    padding: 1rem;
    max-height: calc(100vh - 150px); /* Adjust based on fixed header height */
    overflow-y: auto;
  }

  .loading-message {
    text-align: center;
    padding: 1rem;
    color: var(--color-text-secondary);
  }
</style>

<div class="recent-notes-container">
  <Documents documents={$recentNotes.notes} />

  {#if $recentNotes.hasMore}
    <div bind:this={endOfResults} class="loading-message">
      {#if loading}
        Loading more notes...
      {:else}
        Scroll down to load more.
      {/if}
    </div>
  {/if}
</div>