<script lang="ts">
  import { onMount } from 'svelte';
  import { showSettingsSection } from '$lib/stores';

  let models: string[] = [];
  let currentModel: string = '';
  let selectedModel: string = '';
  let isLoading = true;
  let errorMessage = '';

  const defaultModel = 'qwen3:0.6b';

  async function fetchModels() {
    isLoading = true;
    errorMessage = '';
    try {
      const response = await fetch('http://localhost:3000/models');
      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      const data = await response.json();
      models = [...data.models, defaultModel].filter((value, index, self) => self.indexOf(value) === index); // Add default and remove duplicates
      currentModel = data.currentModel;
      selectedModel = currentModel;
    } catch (error: any) {
      console.error('Error fetching models:', error);
      errorMessage = 'Failed to load models. Please ensure Ollama is running.';
    } finally {
      isLoading = false;
    }
  }

  async function saveModel() {
    isLoading = true;
    errorMessage = '';
    try {
      const response = await fetch('http://localhost:3000/models', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ model: selectedModel }),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }
      currentModel = selectedModel;
      // No alert needed as per user feedback
    } catch (error: any) {
      console.error('Error saving model:', error);
      errorMessage = `Failed to save model: ${error.message}`;
    } finally {
      isLoading = false;
      showSettingsSection.set(false); // Close the modal after saving
    }
  }

  // Function to handle cancel action
  function cancelSettings() {
    showSettingsSection.set(false); // Simply close the modal
  }

  // Fetch models when the component mounts
  onMount(() => {
    fetchModels();
  });

  // Re-fetch models whenever the settings section becomes visible
  $: if ($showSettingsSection) {
    fetchModels();
  }
</script>

<div class="settings-modal">
  <div class="settings-content">
    <button class="close-button" on:click={() => showSettingsSection.set(false)}>
      &times;
    </button>
    <h3>Model Settings</h3>

    {#if isLoading}
      <p>Loading models...</p>
    {:else if errorMessage}
      <p class="error">{errorMessage}</p>
    {:else}
      <div class="form-group">
        <label for="model-select">Select Completion Model:</label>
        <select id="model-select" bind:value={selectedModel}>
          {#each models as model}
            <option value={model}>{model}</option>
          {/each}
        </select>
      </div>
      <p>Current Model: {currentModel}</p>
      <div class="button-group">
        <button on:click={saveModel}>Save Model</button>
        <button on:click={cancelSettings} class="cancel-button">Cancel</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-modal {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
  }

  .settings-content {
    background-color: white;
    padding: 20px;
    border-radius: 8px;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
    position: relative;
    width: 400px;
    max-width: 90%;
  }

  .close-button {
    position: absolute;
    top: 10px;
    right: 10px;
    background: none;
    border: none;
    font-size: 1.5em;
    cursor: pointer;
  }

  .form-group {
    margin-bottom: 15px;
  }

  label {
    display: block;
    margin-bottom: 5px;
    font-weight: bold;
  }

  select {
    width: 100%;
    padding: 8px;
    border: 1px solid #ccc;
    border-radius: 4px;
  }

  .button-group {
    display: flex;
    gap: 10px;
    margin-top: 15px;
  }

  button {
    background-color: #007bff;
    color: white;
    padding: 10px 15px;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1em;
  }

  button:hover {
    background-color: #0056b3;
  }

  .cancel-button {
    background-color: #6c757d;
  }

  .cancel-button:hover {
    background-color: #5a6268;
  }

  .error {
    color: red;
  }
</style>