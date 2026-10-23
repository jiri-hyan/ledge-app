// Typy snapshotu a IPC jsou ve složce `src/types/` (podle oblastí, `types/index.ts` je re-exportuje).
// Tenký re-export tu zůstává, protože část souborů importuje výslovně `types.ts` (Node, `--experimental-strip-types`).
export * from "./types/index.ts";
