<script lang="ts">
import { showNewNoteSection } from '$lib/stores';

function toggleNewNoteSection() {
  showNewNoteSection.update((n: boolean) => !n);
}
</script>

<button id="fab" class="btn">+</button>