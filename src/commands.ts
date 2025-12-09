import { invoke, InvokeArgs } from "@tauri-apps/api/core";

/** An interface connecting the frontend and the backend via commands. */
class Invoker {
  /** Helper function to invoke and cast a type. */
  static async _invoke<T>(
    cmd: string,
    args?: InvokeArgs | undefined
  ): Promise<T> {
    return (await invoke(cmd, args)) as T;
  }

  /** Invoke the `search` command. */
  static async search(key: string, isAppend: boolean): Promise<string[]> {
    return this._invoke<string[]>("search", { key, isAppend });
  }

  /** Invoke the `get_state` command. */
  static async getState(): Promise<FuzzerState> {
    return this._invoke<FuzzerState>("get_state");
  }
}

/**
 * Represents the state of the fuzzer in the backend.
 */
export enum FuzzerState {
  Initialized = "Initialized",
  Uninitialized = "Uninitialized",
}

/**
 * Perform a search to look for items matching the provided key.
 * @param key - The search key.
 * @param priorKey - The prior search key, if any. Used for optimizations.
 * @returns An array of matching item strings.
 */
export async function search(
  key: string,
  priorKey: string | null = null
): Promise<string[]> {
  const isAppend = priorKey !== null && key.startsWith(priorKey);
  return await Invoker.search(key, isAppend);
}

/**
 * Get the current state of the fuzzer.
 * @returns The current state of the fuzzer.
 */
export async function getState(): Promise<FuzzerState> {
  return await Invoker.getState();
}
