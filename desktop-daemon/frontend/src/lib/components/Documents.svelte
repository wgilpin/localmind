<script lang="ts">
  export let documents: Array<{ chunk_text: string }> = [];
</script>

<div class="documents-container">
  {#if documents.length > 0}
    <h2>Retrieved Documents:</h2>
    <ul>
      {#each documents as doc}
        <li>
          <p>{doc.chunk_text}</p>
        </li>
      {/each}
    </ul>
  {:else}
    <p>No documents retrieved.</p>
  {/if}
</div>

<style>
  .documents-container {
    margin-top: 1rem;
    padding: 1rem;
    background-color: #f0f0f0;
    border-radius: 8px;
  }

  ul {
    list-style: none;
    padding: 0;
  }

  li {
    background-color: #e0e0e0;
    margin-bottom: 0.5rem;
    padding: 0.5rem;
    border-radius: 4px;
  }
</style>