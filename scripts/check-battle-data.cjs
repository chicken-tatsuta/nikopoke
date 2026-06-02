#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const engineDataDir = path.join(repoRoot, 'engine-rust', 'data');

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

async function main() {
  const movesPath = path.join(engineDataDir, 'moves.json');
  const learnsetsPath = path.join(engineDataDir, 'learnsets.json');
  const moves = readJson(movesPath);
  const learnsets = readJson(learnsetsPath);
  const wasm = await import(path.join(repoRoot, 'engine-rust', 'pkg', 'engine_rust.js'));
  const wasmBytes = fs.readFileSync(path.join(repoRoot, 'engine-rust', 'pkg', 'engine_rust_bg.wasm'));
  await wasm.default({ module_or_path: wasmBytes });

  const moveIds = new Set(Object.keys(moves));
  const unknown = [];
  const wasmUnsupported = [];

  for (const [speciesId, speciesMoves] of Object.entries(learnsets)) {
    const seen = new Set();
    for (const moveId of speciesMoves) {
      if (!moveIds.has(moveId)) {
        unknown.push({ speciesId, moveId });
      }
      if (seen.has(moveId)) {
        unknown.push({ speciesId, moveId, reason: 'duplicate_move_in_species' });
      }
      seen.add(moveId);

      // Verify the move id is accepted by the currently bundled WASM.
      // If not accepted, createCreature silently falls back to default moves,
      // so we must inspect the returned move list.
      let creature = null;
      try {
        creature = wasm.createCreature(speciesId, { moves: [moveId] });
      } catch {
        creature = null;
      }
      const returnedMoves = Array.isArray(creature?.moves) ? creature.moves : [];
      if (!returnedMoves.includes(moveId)) {
        wasmUnsupported.push({ speciesId, moveId, returnedMoves: returnedMoves.slice(0, 4) });
      }
    }
  }

  if (unknown.length === 0 && wasmUnsupported.length === 0) {
    console.log('Battle data check passed.');
    console.log(`- moves: ${moveIds.size}`);
    console.log(`- species: ${Object.keys(learnsets).length}`);
    return;
  }

  if (unknown.length > 0) {
    console.error(`Unknown move IDs in learnsets: ${unknown.length}`);
    for (const row of unknown.slice(0, 100)) {
      console.error(`- ${row.speciesId}: ${row.moveId}${row.reason ? ` (${row.reason})` : ''}`);
    }
  }

  if (wasmUnsupported.length > 0) {
    console.error(`Moves not accepted by bundled WASM: ${wasmUnsupported.length}`);
    for (const row of wasmUnsupported.slice(0, 100)) {
      console.error(`- ${row.speciesId}: ${row.moveId} (fallback: ${row.returnedMoves.join(', ')})`);
    }
  }

  process.exit(1);
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
