import { defineConfig } from 'vite';
export default defineConfig({
  server: {
    //options for the dev server (npm run dev)
    fs: {
      //its rules for serving files from disk
      allow: [
        //the folders its allowed to serve
        '.', // the project itself (web/)
        '../crates/engine/pkg', //the (engine) wasm-pack output
      ],
    },
  },
});
