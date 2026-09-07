// Vite's defaults, plus the one thing this tree needs to say out loud: the generated
// TypeScript model types live in `schema/`, which is outside this package's root, so the dev
// server has to be allowed to read them. `@escribass/schema` is a `file:` link, not a
// registry package, and §4.1 forbids the alternative — a hand-written copy of the model here.
import { defineConfig } from "vite";

export default defineConfig({
  server: { fs: { allow: [".."] } },
});
