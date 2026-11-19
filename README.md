Alien Defense!

Alien Defense is a browser-based shooter game, inspired by my grade-school
days where I ditched school to crash the local retro arcade with Dad.

Alien Defense's game engine is written entirely in Rust and compiled to WebAssembly
with wasm-pack. The rest of Alien Defense is in TypeScript.
The Vite build tool handles the development server for the game.

I've broken down the game development into 8 steps (M0-M7), and it
practically combines what I've learned at UC Berkeley along with
the general software development techniques I've come across while
in industry so far.

See [docs/SPEC.md](docs/SPEC.md) for the full game rules and engine design.

So far: project setup is underway, my Rust -> WASM pipeline is working, CI is next. (11/18/2026)

The game itself (in a nutsell):

	- You play as a UFO with missile firing capability
	- 
Technical highlights:
	- Rust + wasm-pack, TypeScript, Vite, Canvas 2D, and the testing tools (cargo test/proptest, Vitest).
	- The deterministic engine is a seeded RNG with fixed steps, using an integer-based grid
	- The same engine running in the browser and (TBD) on the server are used to verify leaderboard scores based on replay
	- The pathfinding that adversarial enemies use follow a myriad of algos (backtracking DFS for the missile-linked chain descending downwards upon the player's UFO, A* path-finding heuristic based search for the gunships targeting the UFO)
	- The overall testing plan for the project, follows example-based + property-based testing paradigms)
 

## Roadmap

- [ ] **M0: Project setup.** Rust → WASM pipeline working (CI next)
- [ ] **M1: Board and HUD.** 28 × 36 grid, side HUD, fixed-step game loop
- [ ] **M2: Player.** UFO movement (keys + mouse) and shooting
- [ ] **M3: Missile chain.** Zig-zag, dives, splitting, DFS around obstacles
- [ ] **M4: Obstacles.** 4-hit clearing, wreckage, per-level recolor
- [ ] **M5: Other enemies.** Jets and rockets, falling enemies, A\* gunship
- [ ] **M6: Game flow.** Levels, lives, scoring, game over
- [ ] **M7: Replays.** Seed + input log reproduces a game exactly

## Repo layout

```
alien-defense/
  -> Cargo.toml            Rust workspace (lists the crates)
  -> rust-toolchain.toml   pinned Rust version
  -> .nvmrc                pinned Node version
  -> crates/
       -> engine/          game rules in Rust, compiled to WebAssembly
  -> web/                  TypeScript app (Vite): rendering, input, game loop
  -> assets/sprites/       pixel-art SVG sprites
  -> docs/SPEC.md          game rules and engine design
```

© 2026 Matthew Zarate. All rights reserved.
