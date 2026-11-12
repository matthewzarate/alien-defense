# Alien Defense! - Project Spec 11/11/2026

## 1. Overview & milestones

**Alien Defense!** is an original browser-based shooter. **Alien Defense!** interpolates classical retro arcade games that I used to play with my Dad at Capcom's **Nickel City** in San José, CA. In **Alien Defense!** you fly a small UFO and defend against a human attack: chains of nuclear missiles snaking down the screen.

For **Alien Defense!'s** technical stack:
- The **game engine** (rules and simulation) is written in **Rust** and compiled to **WebAssembly** with wasm-pack. It is a pure, deterministic `step(state, input) -> state` and knows nothing about drawing.
- Everything else in **Alien Defense!** is **TypeScript**: Canvas 2D rendering, input, game-loop timing, the HUD, and menus. Later it also covers the leaderboard API and the MCP server.
- **Vite** handles the dev server and build. Tests use `cargo test` + `proptest`, Vitest, and later Playwright. GitHub Actions runs CI/CD and deploys to Cloudflare.

### Milestones

| # | Milestone | Done when |
|---|---|---|
| M0 | Project setup | Vite + TypeScript + wasm-pack build runs locally, and CI typechecks, lints, tests and builds on every push/PR. |
| M1 | Board, HUD and render loop | The 28 × 36 board and the side HUD (title, score, lives, level, credits) draw from engine state at any window size. A fixed-step loop calls `step`. |
| M2 | Player and shooting | The UFO moves inside the player zone with keys and mouse, and a single shot fires per Space press. |
| M3 | Missile chain | The chain zig-zags and dives, splits when shot, finds a way around obstacles with DFS, cuts through when enclosed, and roams the player zone. |
| M4 | Obstacles | Level-start spawn, 4-hit clearing, wreckage, repair on life loss, and the per-level recolor wave. |
| M5 | Other enemies | Crossers, the falling enemy, and the gunship using A*. |
| M6 | Lives, levels, scoring | The level ends when every link is destroyed. Lives and extra lives work, the chain grows with level, and game over works. |
| M7 | Save/Load and replay | A game is stored as seed + config + per-frame input log, and replaying it reproduces the same game exactly. |
| — | Later phases | See [§6](#6-later-phases). |

---

## 2. Game rules

### 2.1 Board

- The grid is **28 columns × 36 rows** of square cells. Row 0, Col 0 is the grid's upper left-hand corner.
- The **player zone** is the **bottom 6 rows** (rows 30–35). The **upper region** is rows 0–29.
- The **bottom row** (row 35) is special. Shots only travel upward, so anything in the UFO's own row can't be shot. For that reason **no obstacle or wreckage may ever be placed in the bottom row.**
- All of these numbers live in one `GameConfig` (see [§3.1](#31-configuration)). We hardcode them so that the game looks and feels consistent across different screens.
- The **HUD sits outside the board, on its side.** It shows the title, score, lives, level and credits. It is rendering only and is not part of the grid.

### 2.2 Position vs. drawing

Every entity has a **position**, a [row,col] on the grid: the cells it occupies in the game logic. All collisions use a position. A sprite can be **drawn larger than [row,col] location that its position is**, but what it looks like never affects the rules.

| Entity | Position | Drawing |
|---|---|---|
| UFO | 1 × 1 | Its side machine parts spill into the neighboring cells. Its two legs touch the bottom of its cell, and its dome about touches the top. |
| Missile link | 1 × 1 | |
| Obstacle / wreckage | 1 × 1 | |
| Shot | 1 × 1 | |
| Crosser (jet / rocket) | **2 × 1** (2 cells long) | |
| Falling enemy | TBD | |
| Gunship | TBD | |

The same "drawn bigger than its current cell position" idea applies to the other sprites where it fits.

### 2.3 Player

- The player is a UFO with a tiny alien pilot. It moves freely within the player zone and can't enter cells that hold obstacles.
- **Controls:** arrow keys and WASD move the UFO, **Space** fires, and **P** pauses (same overlay as auto-pause, see [§2.9](#29-undefined-behavior-core-input-only)). With a mouse or trackpad, the UFO follows the cursor within the zone, and Space still fires. Phone controls come later.
- **One shot on screen at a time**, and one shot per press. The shot travels straight up.
- The player chooses a UFO avatar from several designs. The choice is cosmetic only.

### 2.4 Lives

- The player starts with **3 lives**.
- A life is lost when a missile link touches the UFO, or when the gunship touches it. A life is also lost whenever any falling (or randomly spawning enemy that enters via a diagonal angle in the player zone) touches the UFO.
- The player gains **+1 life every 20,000 points**, up to a **cap of 5** lives at once.
- At 0 lives the game is over.
- **On losing a life:**
  - every **damaged** obstacle still on the board is repaired to full health;
  - **every obstacle stays where it is**: destroyed obstacles stay gone, and wreckage and obstacles dropped by falling enemies remain;
  - the chain resets to the level's starting size.

### 2.5 Missile chain

- The chain is a line of linked nuclear missiles led by a **head**.
- **Spawn:** the head enters from the top row at a column chosen by the seeded RNG, and immediately starts moving left or right (also chosen by the seeded RNG). The **allowed spawn columns** are read straight from the board: they are the currently free cells of the top row (`board[0]`). The board is the single source of truth, so the set changes as the game goes: a shot-away obstacle frees its column again, and a new obstacle blocks its column. At spawn time, the RNG picks from the free columns. If none are free, the head spawns anyway and **cuts through** (see below). The free columns are gathered in a stable order (column order) so that picks are deterministic. *(How to store them: see [§3.10](#310-explore-during-implementation); e.g. a `Vec<usize>`.)*
- **Movement:** using the seeded RNG, it alternates between
  - a **horizontal zig-zag**, which drops a row and reverses at walls or obstacles, and
  - a **straight vertical dive**.
- **Shooting a link:**
  - the link is removed, and **wreckage** (an obstacle) appears in its cell;
  - the chain **splits** there;
  - the missile right behind the wreckage becomes a **new head**;
  - that rear piece dives straight down to the bottom, then resumes zig-zagging.
- **Blocked dive:** if a head's dive is blocked, it uses a **backtracking DFS** to find a way around and keep heading down. This applies on every difficulty.
- **Enclosed:** if the DFS finds **no path**, the head **cuts through** obstacles and descends to the player zone. The obstacles it passes through are left untouched: no damage and no removal. Cutting through happens only when the DFS fails.
- **Above the player zone, the chain never moves upward.**
- **In the player zone:** once a head reaches the zone, it **stays there for good**. It roams left, right, up and down within the zone until its links are destroyed, and never climbs back above the zone. Reaching the bottom does **not** cost a life.

### 2.6 Obstacles

- There are two kinds: **alien crystals** on some levels and **crater asteroid rocks** on others.
- Every obstacle, wreckage included, takes **4 hits** to clear.
- Obstacles block the UFO.
- **At level start**, the random obstacle spawn places obstacles **only in the upper region**, never in the player zone.
- Wreckage, and obstacles dropped by the falling enemy, **may** land in the player zone, but **never in the bottom row**.

### 2.7 Levels

- A level ends only when **every missile link is destroyed**.
- Each level spawns a set number of new random obstacles. Existing obstacles carry over.
- Each level has its own colors for obstacles and missiles. Obstacles are recolored **one at a time**: each one changes instantly, so the player sees a wave sweep across the board. This is presentation only and is driven by the level number.
- The chain gains **+1 link every 5 levels** and speeds up **every 5–10 levels**. The exact numbers are TBD and will be tuned with the headless simulator.
- **Normal vs. Hard:** both modes use the same rules and algorithms. In Hard, the chain moves faster, with speed counted in frames per move.

### 2.8 Other enemies

**Crossers (jets and rocket ships)**
- They fly across the upper region, left to right or right to left, and **never enter the player zone**.
- Each one is **2 cells long**, which makes it easier to hit.
- For now they are worth bonus points only.
- They may drop **bombs that fall straight down** from where the crosser is. Obstacles do not block bombs. If the UDO is hit by a bomb, a life is deducted.

**Falling enemy (satellite, sometimes a paratrooper)**
- It drops straight down from the top.
- As it falls, it can randomly drop obstacles and also drop obstacles into the player zone, but never into the bottom row.
- When it reaches the bottom row, it disappears.
- Other behavior is TBD.

**Gunship (bottom-zone attacker, sprite `heli-A-gunship`)**
- **Entry:** it enters the player zone from the left or right at a random point, flying a **diagonal aimed with A\*** at the UFO's current position and current movement.
- **Later steps:** each step randomly uses either A* or a plain 45° diagonal (or a series of vertical hops), so it isn't too hard early on.
- **At the base row:** it randomly either goes straight up, or bounces onto an up-diagonal (up-right or up-left, depending on the side it came from).
- **At the top of the zone:** it dives down-diagonal again, or does vertical hops.
- **Exit:** it keeps going until it reaches the opposite side, then disappears.
- It **cuts through every obstacle** for its whole trip.
- **1 hit** destroys it. **Touching it costs the player a life.**
- Its rotor animation (3 frames) is render-only and is not part of the engine state.

### 2.9 Undefined behavior (core input only)

This list is deliberately short. Convoluted key combinations are out of scope.

- **Window loses focus or tab is hidden:** the game **auto-pauses** and shows a "Paused" overlay. The gamee resumes only when the player presses a key or clicks; it never resumes by itself. The press that resumes is used up: resuming with **Space** does **not** fire. Pausing happens entirely in TypeScript: while paused, `step` isn't called, so no frames pass and replays are unaffected.
- **Window resize:** affects rendering only. The cell pixel size is recomputed, and the engine is unaffected.
- **Opposite direction keys held together** (e.g. ← and →, or A and →): **the last key pressed wins**. Releasing it falls back to the other key if that one is still held. TypeScript resolves this before input reaches the engine. This is so that the engine (and the input log) only ever sees and records one direction or none per axis.
- **Keyboard and mouse at the same time:** TBD.

### 2.10 Scoring

TBD.

---

## 3. Engine design

This section says **what** each part of the engine must do and **why**. This engine design does not fix which Rust types to use. Each subsection ends with **Ideas to research**: candidate data structures and approaches, including alternatives. I am doing this so that the trade-offs can be weighed during implementation. This section and whatever examples are in them are just starting points. The open choices are collected in [§3.10](#310-explore-during-implementation).

### 3.1 Configuration

- We hold every dimension and tuning number in one configuration: 28 columns, 36 rows, a 6-row allowable player zone, hits per obstacle, life rules, chain lengths and speeds, spawn rates, and so on.
- The rest of the game's mechanics are **derived** from the 1 configuration (for example, the zone's top row = rows − zone rows; the bottom row = rows − 1). These are essentially hardcoded numbers I've decided before implementation, I am aiming for abstraction and modularity.
- The configuration is **checked when it's created** and rejects nonsense (for example, a zone of fewer than 2 rows, or a zone taller than the board).
- **Every replay record** follows this 1 configuration. We do this for replays.

**Ideas to research:** a plain struct (e.g. `GameConfig`) with a constructor that validates; `const` values vs. a runtime value; how the configuration gets serialized into a replay record.

### 3.2 Board as a grid graph

- The board is a grid of **cells**, each addressed by (row, col). A cell knows what's in it: empty, an obstacle with its remaining hits, missile links, the UFO, and so on.
- **During a cut-through, a missile link and an obstacle share a cell**, so the representation must allow that.
- **Graph view:** each cell is a node, and edges go to its neighbors. Which neighbors count depends on who is moving:
  - the chain above the zone: **down, left and right only** (it can never move up);
  - the chain inside the zone: **all 4 directions**;
  - the gunship: **8 directions**, including diagonals.
- **Neighbor order must be deterministic.** For the chain's DFS, the head always tries **down first**, then the **seeded RNG** picks whether it tries left or right next.

**Ideas to research:**
- One flat list of cells in row-major order (`Vec<Cell>` with index = row × cols + col) vs. a list of rows (`Vec<Vec<Cell>>`) vs. a fixed-size 2D array.
- What a `Cell` is: several true/false flags vs. an `enum` (which can't hold impossible combinations) vs. **separate layers** (an obstacle grid plus lists of entities with positions). Think about how each handles the cut-through case.
- An explicit graph (adjacency lists) vs. an **implicit** graph (compute neighbors on the fly from (row, col)).

### 3.3 Chain as a splittable list

- The board holds any number of chains. Each chain is an **ordered list of links, head first**.
- **Moving (follow the leader):** the head steps to its next cell, and every other link moves into the cell of the link in front of it.
- **Splitting** when a link is shot:
  - the chain is cut into a **front part** (links before the shot one) and a **rear part** (links after it);
  - the shot link's cell becomes wreckage;
  - the rear part's first link becomes a **new head**, and that piece switches to diving.
- **Invariant:** remaining links = starting links − destroyed links.

**Ideas to research:**
- `Vec` (move = shift every position) vs. `VecDeque` (move = add a cell at the front, drop one at the back) vs. a linked list (and why linked lists are awkward in Rust).
- Splitting a collection in two: look up `split_off` on `Vec` and `VecDeque`.
- Storing link **positions** vs. storing link **objects**; how a shot finds which chain and which link it hit.
- The Big-O of a move and a split for each option, and whether it matters for chains of 10–20 links.

### 3.4 Backtracking DFS for blocked dives

- Modeled on recursive maze backtracking in general, also discussed from Stanford CS 106B.
- **Goal:** find a path from the head down to the zone's top row, using only the chain's allowed moves (no upward moves above the zone).
- **Method:** try neighbors in the order from [§3.2](#32-board-as-a-grid-graph), remember visited cells, and back up when a branch dead-ends. The search succeeds when it reaches the goal row.
- It **must always finish** and clearly report "no path". Then the head cuts through (see [§2.5](#25-missile-chain)).
- **Size:** at most 28 × 30 = 840 cells above the zone. Visiting each cell at most once keeps a search O(V + E).
- **Re-search policy:** recompute the path every step, or keep it until the board changes? **(TBD)**

**Ideas to research:** recursion vs. an explicit stack (`Vec` used as a stack); how to record visited cells (a grid of booleans vs. a set); how to return the path so the chain can follow it.

### 3.5 A* for the gunship

- The write-up is modeled on Princeton COS 226 "8 Puzzle".
- **Search space:** the cells of the player zone (28 × 6 = 168), with 8-direction moves that avoid obstacles.
- **Target:** the UFO's cell predicted **one step ahead**: its current cell plus its current direction of movement. If the UFO isn't moving, that's just its current cell. If the predicted cell is outside the zone or inside an obstacle, the target falls back to the UFO's current cell.
- **Heuristic:** it must **never overestimate** the remaining distance, so A* still finds a shortest path. With diagonal moves costing the same as straight ones, the Chebyshev distance `max(|Δrow|, |Δcol|)` qualifies.
- **Ties** are broken in a fixed order, which keeps the search deterministic.

**Ideas to research:** a priority queue as a **binary heap** (`BinaryHeap`, a max-heap, and how to turn it into a min-heap with `Reverse` or a custom `Ord`) vs. a sorted list vs. a plain scan for the minimum (fine for 168 cells?); how to store "came from" links to rebuild the path; the Chebyshev vs. Manhattan vs. octile heuristics.

### 3.6 Seeded RNG

- All randomness comes from one seeded generator. The same seed must always give the same sequence, on every machine and forever, so its algorithm must never change underneath us.
- Choosing the algorithm is TBD.

**Ideas to research:** writing a small generator myself (e.g. xorshift, PCG, SplitMix) vs. a crate pinned to an exact version. This models one shared stream vs. **separate streams** for level generation and for enemy behavior.

### 3.7 Enemy state machines

- Each enemy is always in exactly one named **state**, and each step moves it from one state to the next. Entering from either side is handled by mirroring.
- Example states (names not final):
  - **Chain head:** ZigZag <--> Dive -> Detour (following a DFS path) -> CutThrough → Roaming (in the zone).
  - **Gunship:** Entering -> Rising / UpDiagonal -> Diving / Hopping -> … -> Exiting.
  - **Falling enemy:** Falling -> Gone.
  - **Crosser:** Flying -> Gone.

**Ideas to research:** Rust `enum`s with data in each variant, and `match` to drive transitions; compare with class hierarchies (as in Java) and with traits (`trait Enemy`); where the per-state data lives.

### 3.8 Collision detection

- Collisions are **location-on-cells**: two things collide when their current location share a cell.
- When several things happen in one step, they resolve in a fixed order **(TBD)**. For example: the player moves -> the shot moves -> shot hits -> enemies move → enemy hits.

**Ideas to research:** checking every pair of entities vs. looking things up through the grid (an occupancy grid); how the choice of cell representation in [§3.2](#32-board-as-a-grid-graph) makes this easier or harder.

### 3.9 Logic separated from rendering

- The engine exposes one step function, `step(state, input) -> state`, plus read-only views of the state for drawing.
- **Input reaches the engine as plain per-frame values in grid terms:** which directions are held, whether fire is held, and, for the mouse, a **target cell**. TypeScript converts pixels to cells before calling `step`. This is so the window size never reaches the engine.
- TypeScript will contain: animation, the HUD, the recolor wave and, avatar art.

**Ideas to research:** what can cross the Rust ↔ WASM ↔ TypeScript boundary cheaply (numbers, typed arrays, serialized data).... bit flags vs. a struct of booleans for input......whether the engine detects a fire *press* from the held bit itself or TypeScript sends "pressed this frame".

### 3.10 Explore during implementation

Open representation choices. Tradeoffs to be discussed.

| Milestone | Question |
|---|---|
| M1 | How is the board stored, and what is a `Cell`? (§3.2) |
| M1 | How does input cross from TypeScript into the engine, and who detects a fire *press*? (§3.9) |
| M2 | How are collisions looked up? (§3.8) |
| M3 | Which collection fits a chain that moves and splits? (§3.3) |
| M3 | Recursive or explicit-stack DFS, and how is "visited" recorded? (§3.4) |
| M3 | How are a chain's free spawn columns gathered in a stable order? (§2.5) |
| M5 | Which priority queue for A*, and how are paths rebuilt? (§3.5) |
| M5 | How are enemy states represented? (§3.7) |
| M0–M1 | Which RNG algorithm, and one stream or several? (§3.6) |

---

## 4. Determinism rules

The engine is **perfectly deterministic** and must follow these rules:

1. All randomness comes from the **seeded RNG**.
2. The game advances in **fixed steps (frames)**.
3. We use only integers for cells and frames per move. No floating point in the rules.
4. **Ties are broken in a fixed order**: search neighbor order, the order enemies update in, and collision resolution.
5. When we depend on a data structure or collection for information about an order, I need to use collections where order is preserved.
6. **Animation is render-only** and is never part of the engine state.
7. The input crossing into the engine is in grid terms only (see [§3.9](#39-logic-separated-from-rendering)).

---

## 5. Testing plan

We use **example-based + property-based testing** together.

- **Example-based (`cargo test`)** checks specific rules, for example:
  - shooting link 3 of 6 leaves wreckage in that cell, and the missile behind it becomes a new head;
  - an obstacle clears on the 4th hit;
  - a fully blocked dive cuts through and leaves the obstacles unchanged;
  - losing a life repairs damaged obstacles, keeps every obstacle in place, and resets the chain to the level's starting size.
- **Property-based (`proptest`)** checks invariants over random boards, seeds, inputs **and configs**:
  - no obstacle is ever in the bottom row;
  - if a path down exists, the DFS finds it;
  - remaining links = starting links − destroyed links;
  - the UFO never leaves the zone or overlaps an obstacle;
  - at most one shot is on screen;
  - lives stay within 0..=5;
  - the chain never moves upward above the zone.
- **ASCII-board fixtures (used this style of test during my Build Your Own World final project implementation from my Data Structures course):** example-based tests describe boards as text, so a test reads like a picture of the situation. A small parser turns the text into a board state, and tests can also compare the board after some steps against an expected picture. Example legend: `.` empty, `#` obstacle at full health, `1`–`3` obstacle with that many hits left, `H` chain head, `o` chain body link, `U` UFO, `^` shot. Fixtures can use a smaller `GameConfig` than 28 × 36 to stay short:

  ```text
  // A diving head (H) blocked by a wall must DFS around the gap at the right.
  o.......
  H.......
  ######..
  ........
  ```

- **Determinism:** the same config + seed + inputs, run twice, give identical state and score.
- **Differential:** the server's replay agrees with the client's game.
- **Vitest** covers the TypeScript side and the TS <-> WASM boundary.
- **Playwright** end-to-end tests come once the prototype works.

---

## 6. Later phases

1. **Leaderboard:** Cloudflare Workers + D1.
   - **Replay-based score checking:** the client submits config + seed + per-frame input log + claimed score. The Worker replays them with the **same Rust engine** (as WASM), and the score is accepted only if the replay matches.
   - Limit: this proves the inputs were legal. Basically not bot-proof
2. **Replay viewer:** watch any game from its input log. This is also useful for debugging.
3. **MCP server:** written in TypeScript with the official MCP SDK. For example, it could let an AI assistant read the leaderboard and stats, or play through tools.
4. **Headless simulator:** a native Rust CLI that runs thousands of bot games to tune chain speed and spawn rates.
5. **Phone controls** and **difficulty modifiers**.

---

## 7. Open questions

- **Crosser bombs:** how often I'll drop them. Bombs will pierce through obstacles. They will cost a life if the player hits them, where to add this in the project code.
- **Gunship and falling-enemy footprints.**
- **Scoring** system and point values.
- **Chain numbers:** starting length, speed per difficulty, and how often the speed goes up.
- **Falling enemy:** spawn rate and how often it drops obstacles.
- **Number of new obstacles** per level.
- **RNG algorithm** and whether to split it into streams.
- **DFS re-search policy**, and the order collisions resolve in.
- **Undefined-behavior items** still marked proposed or TBD.
- **Final sprite choices**, the 4th damage stage for obstacles, and reworking the sprites to the new cell size.

---

## References

These are references for concepts only. No code is copied from them.


- **Some spec details I came across as an enrolled student at UC Berkeley from the following courses: EECS 126, CS 61A, CS 61B, CS 61C, and CS170.
- **Princeton COS 226, "8 Puzzle" assignment:** the model for the A* write-up (priority queue, admissible heuristic) used by the gunship.
- **Stanford CS 106B, recursive backtracking (maze solving):** the model for the chain's backtracking DFS.
- An **astrologer** named **Walter Pullen**, whose maze & labyrinth algorithmic work I've read (even before the time I entered UC Berkeley as an undergraduate), located at https://www.astrolog.org/home.htm 
- **RyiSnow, "How to Make a 2D Game in Java" YouTube series** (https://www.youtube.com/playlist?list=PL_QPQmz5C6WUF-pOQDsbsKbaBZqXj4qSq): a reference for the fixed-rate game loop, derived screen constants, key-state input, collision boxes smaller than sprites, sprite-frame animation, text-file maps (the inspiration for the ASCII-board fixtures), and game states (title / play / pause). Alien Defense keeps its engine pure (no drawing, enums and state machines instead of class hierarchies), so it borrows these ideas, not the structure.
