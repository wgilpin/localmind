<script>
let { message = '', type = 'info', duration = 5000, onClose } = $props();

let show = $state(false);

$effect(() => {
    setTimeout(() => {
        show = true;
    }, 10);

    if (duration > 0) {
        setTimeout(() => {
            show = false;
            setTimeout(() => {
                onClose?.();
            }, 300);
        }, duration);
    }
});

function close() {
    show = false;
    setTimeout(() => {
        onClose?.();
    }, 300);
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
</script>

<div class="toast {type}" class:show>
    {@html escapeHtml(message).replace(/\n/g, '<br>')}
    <button class="close-btn" onclick={close}>×</button>
</div>

<style>
.toast {
    background: #374151;
    color: white;
    padding: 12px 16px;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    font-size: 14px;
    line-height: 1.4;
    transform: translateX(100%);
    opacity: 0;
    transition: all 0.3s ease-in-out;
    position: relative;
    padding-right: 40px;
    margin-bottom: 10px;
}

.toast.show {
    transform: translateX(0);
    opacity: 1;
}

.toast.info {
    background: #3b82f6;
}

.toast.warning {
    background: #f59e0b;
}

.toast.error {
    background: #ef4444;
}

.toast.success {
    background: #10b981;
}

.close-btn {
    position: absolute;
    top: 8px;
    right: 8px;
    background: none;
    border: none;
    color: white;
    cursor: pointer;
    font-size: 16px;
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0.7;
}

.close-btn:hover {
    opacity: 1;
}
</style>
