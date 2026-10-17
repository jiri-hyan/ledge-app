// Most k Rustu pro Agent View (`commands/agents.rs`): akce nad session na pozadí a spuštění nové
// (`claude --bg`). Jen na výslovný klik; Rust ověří id i akci proti řádku. Chyby = text pro `sayError`.
import { invoke } from "@tauri-apps/api/core";
import { inTauri } from "../api";
import type { AgentOut, JobAction } from "../types";

const call = <T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> => (inTauri ? invoke<T>(cmd, args) : Promise.resolve(undefined));

export const agentApi = {
  /** `id` = id řádku session; `confirmed` = člověk potvrdil (Zastavit / Odebrat / Restartovat) */
  action: (id: string, action: JobAction, confirmed: boolean) => call<AgentOut>("agent_action", { id, action, confirmed }),
  /** nová session na pozadí ve složce projektu; vrací hlášku */
  spawn: (projectId: string, prompt: string, name: string | null) => call<string>("agent_spawn", { projectId, prompt, name }),
};
