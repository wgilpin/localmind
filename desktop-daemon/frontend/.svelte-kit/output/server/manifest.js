export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set([]),
	mimeTypes: {},
	_: {
		client: {start:"_app/immutable/entry/start.B-pGmLPN.js",app:"_app/immutable/entry/app.Bl4Lwpvm.js",imports:["_app/immutable/entry/start.B-pGmLPN.js","_app/immutable/chunks/B2wwTmVY.js","_app/immutable/chunks/CBJthVVo.js","_app/immutable/chunks/YhXVieLi.js","_app/immutable/entry/app.Bl4Lwpvm.js","_app/immutable/chunks/CBJthVVo.js","_app/immutable/chunks/IHki7fMi.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js')),
			__memo(() => import('./nodes/2.js'))
		],
		routes: [
			{
				id: "/",
				pattern: /^\/$/,
				params: [],
				page: { layouts: [0,], errors: [1,], leaf: 2 },
				endpoint: null
			}
		],
		prerendered_routes: new Set([]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
