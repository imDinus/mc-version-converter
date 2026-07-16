# MC Version Converter

Converts Minecraft: Java Edition worlds saved in **26.1 or later** into a structure that
**older versions (1.21.x down to 1.18.2)** can open. Upgrading (old → new) is planned
as a future direction.

Starting with 26.1, the world save structure was reorganized (per-dimension `dimensions/`
folders, level.dat split into separate files, and more). Once a world is opened in 26.x,
older versions can no longer load it. MC Version Converter reverses that reorganization.

> **Important**: The original world is never modified — results are always written to a
> new folder. Still, back up your world before converting, and test the result with a copy.
> This is an unofficial tool, not affiliated with Mojang or Microsoft.

## Status

**M2 complete**: 26.x worlds can be converted to any version from 1.21.11 down to 1.18.2.

| Milestone | Target | Scope | Status |
|---|---|---|---|
| M1 | 1.21.x | folder remapping + level.dat rebuild + DataVersion rewrite | done (verified in game: 26.2 → 1.21.11) |
| M2 | 1.20.x – 1.18.x | legacy NBT reversal (item format, equipment, attributes, signs) | done |
| M3 | — | bundled per-version block allowlists, block replacement options | planned |
| M4 | — | desktop GUI (drag & drop) | planned |

The M2 reversal converts, when the target version requires it:
- item stacks `{count, components}` → `{Count, tag}` (enchantments, damage, custom
  names, lore, repair cost, unbreakable, shulker box contents, block entity data
  such as copied command blocks, spawn eggs, bucket mobs, maps, lodestone compasses,
  potions, player heads, books, dyed colors, crossbow projectiles)
- entity `equipment` → `HandItems`/`ArmorItems`/`SaddleItem`, `fall_distance` → `FallDistance`
- attribute ids back to their prefixed names (`generic.max_health` etc.)
- sign text back to the pre-1.20 `Text1`–`Text4` format
- plain text (mob name tags, sign lines, item names/lore, book pages) back to the
  JSON text format expected before 1.21.5
- player bed respawn `respawn` compound → `SpawnX/Y/Z/Angle/Dimension/Forced` (pre-1.21.5)
- stored light data is stripped so the target version recalculates lighting on first
  load (prevents dark patches on terrain)

1.17.x and below are not supported because the world height differs (-64..320 vs 0..255).

## Getting started

This tool is distributed as source code — you build it once, then use it like any
other program. One-time setup:

1. **Install Rust** from [rustup.rs](https://rustup.rs) (keep the default options,
   then restart your terminal).
2. **Get the source**: click the green **Code** button on this page → **Download ZIP**
   and extract it anywhere — or `git clone https://github.com/imDinus/mc-version-converter.git`.
3. **Build it**: open a terminal in the project folder and run
   ```
   cargo build --release
   ```
   The first build downloads dependencies and takes a few minutes.
4. Done. From now on, just double-click **`run.bat`**.

## Getting started

This tool is distributed as source code — you build it once on your machine.

**Requirements:**
- [Rust](https://rustup.rs/) — install once via rustup (the default/stable toolchain is fine)
- Windows for `run.bat` (the `mcconvert` CLI itself also works on Linux/macOS)

**Setup (one time):**

1. Get the source: `git clone https://github.com/imDinus/mc-version-converter.git`
   — or click **Code → Download ZIP** on GitHub and extract it.
2. Open a terminal in the project folder and build:

   ```
   cargo build --release
   ```

   The first build downloads dependencies and takes a few minutes.
3. Done. From now on, just double-click `run.bat`.

## Usage

Double-click `run.bat` to open a command window, then type commands there.

```
# convert every world in the input_worlds folder
mcconvert batch <version>

# show a world's version and layout
mcconvert info "world path"

# list supported versions
mcconvert versions
```

`batch` converts every world placed (as a folder) inside `input_worlds/` and writes the
results to `output_worlds/` as `<world name> (<version>)`. Both folders are created on
first run if missing.

Notes on `batch`:
- You can put multiple worlds in `input_worlds/` — they are all converted one after
  another, each into its own output folder. Folders without a `level.dat` are ignored.
- If one world fails, the others still convert; the run then ends with error E60 and
  a per-world FAILED message.
- Existing results are **never overwritten**: if the output folder for a world already
  exists and is not empty, that world is rejected with error E50. Delete the old
  output folder first to convert that world again.

Progress logs (`[LOG] ...`) are printed during conversion. Pass `-q` to suppress them.

Blocks that do not exist in the target version are **replaced with air (minecraft:air)**.
Replacement requires a block allowlist for the target version (`--block-table blocks.json`,
a JSON array like `["minecraft:stone", ...]`). If omitted, replacement is skipped.

## Error codes

| Code | Exit code | Meaning |
|---|---|---|
| E10 | 10 | file I/O error |
| E20 | 20 | NBT parsing/processing error |
| E30 | 30 | world format error (missing level.dat, not a 26.x world, etc.) |
| E40 | 40 | unsupported target version |
| E50 | 50 | output folder error (already exists and is not empty) |
| E60 | 60 | partial batch failure |

## Architecture

```
crates/
├── core/   conversion engine library (mcconvert-core)
│   ├── version   version ↔ DataVersion table
│   ├── world     world folder layout detection/remapping
│   ├── nbt       NBT I/O, level.dat rebuild, data file policies
│   ├── chunk     per-chunk .mca rewriting (parallel)
│   ├── mapping   block allowlist, unsupported block → air
│   ├── pipeline  conversion orchestration, batch mode
│   └── logging   progress logs
└── cli/    command-line interface (mcconvert)
```

## Known limitations

- Blocks/items/mobs added in 26.x do not exist in older versions and are lost on
  conversion (blocks are replaced with air).
- 26.x data with no legacy equivalent is dropped: stopwatches, the End world clock,
  chunk tickets, and world borders of dimensions other than the overworld
  (reported as `[WARN]` during conversion).
- Detailed NBT of mobs/items added in 26.x may be ignored by older versions.
- For targets below 1.20.5: item components without a legacy equivalent and attribute
  modifiers are dropped; below 1.20 sign back-side text is dropped. Status effects are
  converted to their legacy numeric ids for targets below 1.20.2 — only effects that do
  not exist in the target version are dropped. Each drop is reported as `[WARN]`.
- Corrupt region/poi files or chunks are skipped with a warning instead of aborting
  the conversion (common when a world was copied while the game was running).
- Oversized chunk files (.mcc) are copied without conversion.
- Always open the converted world from a copy to verify it in game.

## License

[MIT](LICENSE). Dependency licenses are listed in
[THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

This repository contains no Mojang code, game data, or assets. The world format
handling was written from scratch using the public documentation on the
[Minecraft Wiki](https://minecraft.wiki).

## Credits

Built with [Claude](https://claude.com), Anthropic's AI assistant — including the
reverse-engineering of the 26.x save format against real world files.
