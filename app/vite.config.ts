import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  define: {
    // wallet-adapter/web3.js dependencies expect Node's `global` to exist
    global: "globalThis",
  },
});
