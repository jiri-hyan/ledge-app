// Okno Ledge Studio (kolo 24): snapshot jádra (`get_state` + událost `state`, Rust ji Studiu posílá jen, když
// je otevřené) a skořápka `Studio.tsx` s vlastním titulkem. Zavření = zničit okno (v klidu nežije nic).
import { onCleanup, onMount } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { api, on, type Snapshot } from "../api";
import { EMPTY_SNAPSHOT } from "../snapshot";
import { Toast } from "../ui/toast";
import Studio from "./Studio";

export default function StudioWindow() {
  const [s, setS] = createStore<Snapshot>(structuredClone(EMPTY_SNAPSHOT));
  const apply = (x: Snapshot | undefined) => x && setS(reconcile(x));
  onMount(() => {
    void api.getState().then(apply, () => {});
    const off = on<Snapshot>("state", apply);
    onCleanup(() => void off.then((f) => f()));
  });
  return (
    <>
      <Studio s={s} windowed initial={location.hash || null} />
      <Toast />
    </>
  );
}
