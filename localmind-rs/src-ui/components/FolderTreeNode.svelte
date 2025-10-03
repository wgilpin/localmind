<script>
import FolderTreeNode from './FolderTreeNode.svelte';

let { node, depth = 0, excludedFolders, onToggle } = $props();
</script>

{#if node.folder}
    <div class="folder-item" style="padding-left: {depth * 20}px">
        <span class="tree-icon">{depth === 0 ? '📂' : '└─'}</span>
        <input
            type="checkbox"
            id="folder-{node.folder.id}"
            checked={excludedFolders.includes(node.folder.id)}
            onchange={() => onToggle(node.folder.id)}
        />
        <label for="folder-{node.folder.id}">
            {node.name}
        </label>
        <span class="folder-count">({node.folder.bookmark_count})</span>
    </div>
{/if}
{#each Object.values(node.children) as child}
    <FolderTreeNode node={child} depth={depth + 1} {excludedFolders} {onToggle} />
{/each}

<style>
.folder-item {
    padding: 0;
    display: flex;
    align-items: center;
    gap: 2px;
}

.tree-icon {
    color: #9ca3af;
    font-size: 0.9rem;
    user-select: none;
    min-width: 20px;
}

.folder-item input[type="checkbox"] {
    cursor: pointer;
    accent-color: #60a5fa;
}

.folder-item label {
    cursor: pointer;
    flex: 1;
    color: #d1d5db;
}

.folder-count {
    color: #9ca3af;
    font-size: 0.9rem;
}
</style>
