// Kontrakt „Poslat do propojené aplikace“ (Rust `connector::plan`, `commands/connector.rs`). Klíč se do UI nikdy nevrací:
// jádro zná jen „uložen / chybí“. Náhled sám nic neodesílá (jen čte seznam členů týmu); odeslání = `connectorSend`.
import { invoke } from "@tauri-apps/api/core";
import type { Plan } from "./model";
import type { Outcome } from "./model";

/** `name` = `connector.name` (může být prázdné), `configured` = adresa `connector.url` je neprázdná a platná. */
export type ConnectorState = { hasKey: boolean; host: string; name: string; configured: boolean };
export type Preview = { plan: Plan; hash: string; host: string; name: string };
export type SendOut = { results: Outcome[]; linkError: string | null };

export const connectorState = () => invoke<ConnectorState>("connector_state");
export const connectorSetKey = (key: string) => invoke<void>("connector_set_key", { key });
export const connectorClearKey = () => invoke<void>("connector_clear_key");
export const connectorPreview = (path: string, mode?: "tasks" | "note") => invoke<Preview>("connector_preview", { path, mode: mode ?? null });
export const connectorSend = (path: string, hash: string, mode: "tasks" | "note", lines: number[]) => invoke<SendOut>("connector_send", { path, hash, mode, lines });
