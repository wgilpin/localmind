<script lang="ts">
	import { createEventDispatcher } from 'svelte';

	export let documents: Array<{ id: string; title: string; content: string;[key: string]: any }> = [];
	let expanded: { [key: number]: boolean } = {};

	$: safeDocuments = Array.isArray(documents) ? documents : [];

	function toggle(index: number) {
		expanded[index] = !expanded[index];
	}

	const dispatch = createEventDispatcher();

	function handleDelete(docId: string) {
		dispatch('delete', docId);
	}

	function handleEdit(docId: string) {
		dispatch('edit', docId);
	}

	function handleKeydown(event: KeyboardEvent, index: number) {
		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			toggle(index);
		}
	}
</script>

<div class="documents-container">
	{#if safeDocuments.length > 0}
		<div class="cards-container">
			{#each safeDocuments as doc, i}
				<div class="card">
					<div
						class="card-header"
						on:click={() => toggle(i)}
						on:keydown={(e) => handleKeydown(e, i)}
						role="button"
						tabindex="0"
					>
						<h3>{doc.title}</h3>
						{#if expanded[i]}
							<div class="card-header-actions">
								<button class="icon-button" on:click|stopPropagation={() => handleEdit(doc.id)} title="Edit">
									✏️
								</button>
								<button class="icon-button delete-button" on:click|stopPropagation={() => handleDelete(doc.id)} title="Delete">
									🗑️
								</button>
							</div>
						{/if}
						<span class="arrow" class:expanded={expanded[i]}></span>
					</div>
					{#if expanded[i]}
						<div class="card-content">
							<p>
								{doc.content}
							</p>
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{:else}
		<p>No documents retrieved.</p>
	{/if}
</div>

<style>
	.documents-container {
		margin-top: 1rem;
		padding: 1rem;
		background-color: #fdf5e6; /* OldLace */
		border-radius: 8px;
		color: #333;
	}
	.cards-container {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.card {
		background-color: #fff8dc; /* Cornsilk */
		border-radius: 4px;
		padding: 1rem;
	}
	.card-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		cursor: pointer;
	}
	.card-header-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}
	h3 {
		margin: 0;
		flex-grow: 1; /* Allow title to take available space */
	}
	.arrow {
		width: 0;
		height: 0;
		border-left: 5px solid transparent;
		border-right: 5px solid transparent;
		border-top: 5px solid #333;
		transition: transform 0.3s;
	}
	.arrow.expanded {
		transform: rotate(180deg);
	}
	.card-content {
		margin-top: 1rem;
	}
	.icon-button {
		background: none;
		border: none;
		font-size: 1.2em;
		cursor: pointer;
		padding: 0.2em;
		transition: transform 0.2s ease;
	}
	.icon-button:hover {
		transform: scale(1.1);
	}
	.delete-button {
		color: #dc3545; /* Red color for delete */
	}
	.delete-button:hover {
		color: #c82333;
	}
</style>