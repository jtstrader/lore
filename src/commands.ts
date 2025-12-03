import { invoke } from "@tauri-apps/api/core";

/**
 * Invoke the backend search command to look for items matching the provided key.
 * @param key - The search key.
 * @param priorKey - The prior search key, if any. Used for optimizations.
 * @returns An array of matching item strings.
 */
export async function search(
  key: string,
  priorKey: string | null = null
): Promise<string[]> {
  let isAppend = priorKey !== null && key.startsWith(priorKey);
  return (await invoke("search", { key, isAppend })) as string[];
}
