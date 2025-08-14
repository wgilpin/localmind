<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { currentSearchTerm } from '$lib/stores';
	import { get } from 'svelte/store';

	export let documents: Array<{ id: string; title: string; content: string; url?: string; distance?: number; [key: string]: any }> = [];
	let expanded: { [key: number]: boolean } = {};

	$: safeDocuments = Array.isArray(documents) ? documents : [];

	async function logResultClick(documentId: string, distance?: number) {
		try {
			const searchTerm = get(currentSearchTerm);
			if (!searchTerm) return; // Only log if we have a search term
			
			await fetch('/log-result-click', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					searchTerm,
					documentId,
					distance: distance || 0
				})
			});
		} catch (error) {
			console.error('Failed to log result click:', error);
		}
	}

	function toggle(index: number) {
		expanded[index] = !expanded[index];
		
		// Log the click when expanding (first click)
		if (expanded[index]) {
			const doc = safeDocuments[index];
			if (doc) {
				logResultClick(doc.id, doc.distance);
			}
		}
	}

	const dispatch = createEventDispatcher();

	function handleDelete(docId: string) {
		dispatch('delete', docId);
	}

	function handleEdit(docId: string) {
		dispatch('edit', docId);
	}

	function handleOpenUrl(url: string | undefined, docId: string, distance?: number) {
		if (url) {
			// Log the URL click
			logResultClick(docId, distance);
			window.open(url, '_blank');
		}
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
								{#if doc.url}
									<button class="icon-button" on:click|stopPropagation={() => handleOpenUrl(doc.url, doc.id, doc.distance)} title="Open URL">
										<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#777" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="feather feather-external-link">
											<path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"></path>
											<polyline points="15 3 21 3 21 9"></polyline>
											<line x1="10" y1="14" x2="21" y2="3"></line>
										</svg>
									</button>
								{/if}
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

	/* Style for SVG icons within buttons */
	.icon-button svg {
		width: 1.2em; /* Match font-size of other icons */
		height: 1.2em; /* Match font-size of other icons */
		vertical-align: middle; /* Align with text-based icons */
	}
</style>