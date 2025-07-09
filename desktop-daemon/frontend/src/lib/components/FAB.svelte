<script lang="ts">
import { showNewNoteSection } from './NewNote.svelte';

function toggleNewNoteSection() {
  showNewNoteSection.update(n => !n);
}
</script>

<button id="fab" class="btn">+</button>