<script lang="ts">
	export let documents: Array<{ title: string; content: string;[key: string]: any }> = [];
	let expanded: { [key: number]: boolean } = {};

	$: safeDocuments = Array.isArray(documents) ? documents : [];

	function toggle(index: number) {
		expanded[index] = !expanded[index];
	}
</script>

<div class="documents-container">
	{#if safeDocuments.length > 0}
		<div class="cards-container">
			{#each safeDocuments as doc, i}
				<div class="card">
					<div class="card-header" on:click={() => toggle(i)}>
						<h3>{doc.title}</h3>
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
	h3 {
		margin: 0;
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
</style>