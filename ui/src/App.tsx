import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main style={{ padding: "2rem", fontFamily: "sans-serif" }}>
      <h1>jamjam</h1>
      <p>P2P Audio Communication</p>

      <form
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
        style={{ marginTop: "2rem" }}
      >
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Enter a name..."
          style={{ padding: "0.5rem", marginRight: "0.5rem" }}
        />
        <button type="submit" style={{ padding: "0.5rem 1rem" }}>
          Greet
        </button>
      </form>

      {greetMsg && <p style={{ marginTop: "1rem" }}>{greetMsg}</p>}
    </main>
  );
}

export default App;
