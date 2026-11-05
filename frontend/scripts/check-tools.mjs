import { build } from 'esbuild';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const directory = await mkdtemp(join(tmpdir(), 'nikipoke-tools-'));
try {
  const outfile = join(directory, 'check.mjs');
  await build({
    stdin: {
      resolveDir: process.cwd(), loader: 'ts', contents: `
        import assert from 'node:assert/strict';
        import species from './public/data/species.json';
        import moves from './public/data/moves.json';
        import learnsets from './public/data/learnsets.json';
        import { pokemonData, moveData } from './src/tools/data/catalog';
        import { POKEMON_PRESETS } from './src/tools/data/pokemonPresets';
        import { calcStat } from './src/tools/lib/stats';
        assert.equal(pokemonData.length, Object.keys(species).length);
        assert.equal(new Set(moveData.map(move => move.id)).size, moveData.length);
        for (const mon of pokemonData) {
          assert.deepEqual(mon.types, species[mon.id].type);
          const expected = new Set(learnsets[mon.id].map(id => moves[id]?.name).filter(Boolean));
          const actual = new Set(moveData.filter(move => move.targets.includes(mon.id)).map(move => move.name));
          assert.deepEqual(actual, expected);
        }
        for (const [id, preset] of Object.entries(POKEMON_PRESETS)) {
          assert.ok(Object.values(preset.evs).every(ev => ev >= 0 && ev <= 32));
          assert.ok(Object.values(preset.evs).reduce((a, b) => a + b, 0) <= 66);
          assert.ok(preset.moveIds.every(name => moveData.some(move => move.id === name && move.targets.includes(id))));
        }
        for (const level of [1, 50, 100]) for (const ev of [0, 1, 31, 32]) {
          const base = Math.floor((80 * 2 + 31) * level / 100);
          assert.equal(calcStat(80, 31, ev, level, 'hp'), base + level + 10 + ev);
          assert.equal(calcStat(80, 31, ev, level, 'spe'), base + 5 + ev);
        }
        console.log('Tools checks passed: catalog, learnsets, presets, and stat formulas');
      `,
    },
    bundle: true, platform: 'node', format: 'esm', outfile,
  });
  await import(pathToFileURL(outfile).href);
} finally {
  await rm(directory, { recursive: true, force: true });
}
