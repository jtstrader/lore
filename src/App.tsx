import { createSignal, onMount } from "solid-js";
import { FuzzerState, getState, search } from "./commands";
import "./App.css";

function App() {
  const [searchResults, setSearchResults] = createSignal([] as string[]);
  const [key, setKey] = createSignal(null as string | null);

  /**
   * Perform an initial search on application mount with an empty key.
   * This should populate the search results with all available items.
   */
  onMount(async () => {
    const state = await getState();
    if (state === FuzzerState.Uninitialized) {
      console.warn("Fuzzer is uninitialized. Searching cannot occur.");
      return;
    }

    await updateKeyAndSearch("");
  });

  /**
   * Update the search key and perform a search.
   * @param newKey The new key to use during the search.
   */
  const updateKeyAndSearch = async (newKey: string) => {
    let results = await search(newKey, key());
    setKey(newKey);
    setSearchResults(results);
  };

  return (
    <main class="container">
      <h1>lore</h1>

      <input
        id="greet-input"
        onChange={(e) => updateKeyAndSearch(e.currentTarget.value)}
        onKeyPress={(e) => updateKeyAndSearch(e.currentTarget.value)}
        onPaste={(e) => updateKeyAndSearch(e.currentTarget.value)}
        onInput={(e) => updateKeyAndSearch(e.currentTarget.value)}
        autoCapitalize="none"
        autocomplete="off"
        autocorrect="off"
        spellcheck="false"
      />

      <p>{searchResults().join(", ")}</p>
    </main>
  );
}

export default App;
