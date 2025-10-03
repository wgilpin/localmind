// Tauri API integration with Svelte 5
let invoke = null;
let listen = null;
let shell = null;
let isInitialized = false;

export function initializeTauriAPI() {
    console.log('Initializing Tauri API...');

    return new Promise((resolve, reject) => {
        setTimeout(() => {
            if (window.__TAURI__) {
                console.log('Found window.__TAURI__');
                console.log('__TAURI__ keys:', Object.keys(window.__TAURI__));

                if (window.__TAURI__.core && window.__TAURI__.core.invoke) {
                    console.log('Found invoke at window.__TAURI__.core.invoke');
                    invoke = window.__TAURI__.core.invoke;
                } else if (window.__TAURI__.invoke) {
                    console.log('Found invoke at window.__TAURI__.invoke');
                    invoke = window.__TAURI__.invoke;
                } else {
                    for (const key of Object.keys(window.__TAURI__)) {
                        console.log(`__TAURI__.${key}:`, typeof window.__TAURI__[key]);
                        if (window.__TAURI__[key] && typeof window.__TAURI__[key] === 'object') {
                            console.log(`__TAURI__.${key} keys:`, Object.keys(window.__TAURI__[key]));
                        }
                    }
                }

                if (window.__TAURI__.event && window.__TAURI__.event.listen) {
                    console.log('Found event.listen at window.__TAURI__.event.listen');
                    listen = window.__TAURI__.event.listen;
                } else {
                    console.warn('Tauri event.listen not found');
                }

                if (window.__TAURI__.shell) {
                    console.log('Found shell at window.__TAURI__.shell');
                    shell = window.__TAURI__.shell;
                } else {
                    console.warn('Tauri shell not found');
                }

                if (invoke) {
                    console.log('Successfully initialized Tauri API');
                    isInitialized = true;
                    resolve({ invoke, listen, shell });
                } else {
                    console.error('Tauri invoke function not found');
                    reject(new Error('Tauri API not properly initialized'));
                }
            } else {
                console.error('window.__TAURI__ not available after timeout');
                reject(new Error('Tauri API not available'));
            }
        }, 100);
    });
}

export function getTauriAPI() {
    return {
        get invoke() { return invoke; },
        get listen() { return listen; },
        get shell() { return shell; },
        get isInitialized() { return isInitialized; }
    };
}
