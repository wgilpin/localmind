<script lang="ts">
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';

    export let showModal: boolean;
    export let document: { id: string; title: string; content: string } | null = null;

    let editedTitle: string = '';
    let editedContent: string = '';
    let modalElement: HTMLElement; // Reference to the modal content div

    // This reactive block will only run when 'showModal' changes or 'document' changes.
    // When the modal becomes visible and a document is provided, initialize the edited fields.
    // This should prevent immediate overwrites during typing.
    $: if (showModal && document) {
        editedTitle = document.title;
        editedContent = document.content;
    } else if (!showModal) {
        // Optionally reset fields when modal closes, though not strictly necessary for this bug
        editedTitle = '';
        editedContent = '';
    }

    const dispatch = createEventDispatcher();

    function handleSave() {
        if (document) {
            dispatch('save', { id: document.id, title: editedTitle, content: editedContent });
            showModal = false;
        }
    }

    function handleClose() {
        showModal = false;
    }

    // Handle keyboard accessibility for closing modal with Escape key
    function handleKeydown(event: KeyboardEvent) {
        if (event.key === 'Escape') {
            handleClose();
        }
    }

    onMount(() => {
        // Add event listener to the document for Escape key
        window.addEventListener('keydown', handleKeydown);
        // Focus the modal when it opens
        if (modalElement) {
            modalElement.focus();
        }
    });

    onDestroy(() => {
        // Remove event listener when component is destroyed
        window.removeEventListener('keydown', handleKeydown);
    });
</script>

{#if showModal}
<div class="modal-overlay">
    <div class="modal-content" role="dialog" aria-modal="true" tabindex="-1" bind:this={modalElement}>
        <div class="modal-header">
            <h2>Edit Document</h2>
            <button class="close-button" on:click={handleClose} aria-label="Close modal">✖️</button>
        </div>
        <label for="edit-title">Title:</label>
        <input id="edit-title" type="text" bind:value={editedTitle} />

        <label for="edit-content">Content:</label>
        <textarea id="edit-content" bind:value={editedContent}></textarea>

        <div class="modal-actions">
            <button on:click={handleSave}>Save</button>
            <button on:click={handleClose}>Cancel</button>
        </div>
    </div>
</div>
{/if}

<style>
    .modal-overlay {
        position: fixed;
        top: 0;
        left: 0;
        width: 100%;
        height: 100%;
        background: rgba(0, 0, 0, 0.5);
        display: flex;
        justify-content: center;
        align-items: center;
        z-index: 1000;
    }

    .modal-content {
        background: #fff;
        padding: 2rem;
        border-radius: 8px;
        box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
        width: 90%;
        max-width: 500px;
        display: flex;
        flex-direction: column;
        gap: 1rem;
        outline: none; /* Remove default focus outline */
    }

    .modal-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        margin-bottom: 1rem;
    }

    .modal-header h2 {
        margin: 0;
        color: #333;
    }

    .close-button {
        background: none;
        border: none;
        font-size: 1.5rem;
        cursor: pointer;
        color: #666;
        padding: 0;
    }

    .close-button:hover {
        color: #333;
    }

    .modal-content label {
        font-weight: bold;
        color: #555;
    }

    .modal-content input[type="text"],
    .modal-content textarea {
        width: 100%;
        padding: 0.5rem;
        border: 1px solid #ddd;
        border-radius: 4px;
        font-size: 1rem;
        box-sizing: border-box; /* Include padding and border in the element's total width and height */
    }

    .modal-content textarea {
        min-height: 150px;
        resize: vertical;
    }

    .modal-actions {
        display: flex;
        justify-content: flex-end;
        gap: 0.5rem;
        margin-top: 1rem;
    }

    .modal-actions button {
        padding: 0.6rem 1.2rem;
        border: none;
        border-radius: 4px;
        cursor: pointer;
        font-size: 1rem;
        transition: background-color 0.2s ease;
    }

    .modal-actions button:first-child {
        background-color: #007bff;
        color: white;
    }

    .modal-actions button:first-child:hover {
        background-color: #0056b3;
    }

    .modal-actions button:last-child {
        background-color: #6c757d;
        color: white;
    }

    .modal-actions button:last-child:hover {
        background-color: #5a6268;
    }
</style>