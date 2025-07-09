

export const index = 2;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_page.svelte.js')).default;
export const imports = ["_app/immutable/nodes/2.1wn-a9QH.js","_app/immutable/chunks/CBJthVVo.js","_app/immutable/chunks/IHki7fMi.js","_app/immutable/chunks/YhXVieLi.js"];
export const stylesheets = [];
export const fonts = [];
