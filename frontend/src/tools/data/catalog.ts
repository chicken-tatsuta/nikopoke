import species from '../../../public/data/species.json';
import moves from '../../../public/data/moves.json';
import learnsets from '../../../public/data/learnsets.json';
import legacyMoves from './moves.json';
import type { SimMove, SimPokemon } from '../lib/simulator';
import type { PokemonType } from '../lib/damage';

export const pokemonData: SimPokemon[] = Object.values(species).map(mon => ({
  id: mon.id, name: mon.name, types: mon.type as PokemonType[], baseStats: mon.baseStats,
}));

const legacyByName = new Map((legacyMoves as SimMove[]).map(move => [move.name, move]));
const sets = learnsets as Record<string, string[]>;
const adaptedMoves: SimMove[] = Object.values(moves).map(raw => {
  const move = raw as { id: string; name: string; type: string; category: SimMove['category']; power?: number | null; accuracy?: number | null; priority?: number; tags?: string[] };
  const legacy = legacyByName.get(move.name);
  return {
    ...legacy,
    id: move.name,
    name: move.name,
    type: move.type as PokemonType,
    category: move.category,
    power: move.power ?? 0,
    accuracy: move.accuracy == null ? null : move.accuracy * 100,
    priority: move.priority ?? legacy?.priority,
    targets: Object.keys(sets).filter(id => sets[id].includes(move.id)),
  };
});

// Older IDs can coexist with their replacements in the battle catalog.
export const moveData = Array.from(adaptedMoves.reduce((result, move) => {
  const existing = result.get(move.name);
  result.set(move.name, existing ? {
    ...existing, targets: [...new Set([...existing.targets, ...move.targets])],
  } : move);
  return result;
}, new Map<string, SimMove>()).values());

const typeLabels: Record<string, string> = {
  normal: 'ノーマル', fire: 'ほのお', water: 'みず', electric: 'でんき', grass: 'くさ',
  ice: 'こおり', fighting: 'かくとう', poison: 'どく', ground: 'じめん', flying: 'ひこう',
  psychic: 'エスパー', bug: 'むし', rock: 'いわ', ghost: 'ゴースト', dragon: 'ドラゴン',
  dark: 'あく', steel: 'はがね', fairy: 'フェアリー',
};

export const speedRoster = Object.values(species).map(mon => ({
  name: mon.name, baseSpeed: mon.baseStats.spe,
  types: mon.type.map(type => typeLabels[type] ?? type).join('/'), ability1: '', ability2: '',
}));
