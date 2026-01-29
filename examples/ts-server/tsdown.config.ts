import { defineConfig } from "tsdown";
import Macros from "unplugin-macros/rollup";

export default defineConfig({
  entry: ["src/server.ts"],
  format: ["esm"],
  outDir: "dist",
  clean: true,
  plugins: [
    Macros(),
  ],
});
