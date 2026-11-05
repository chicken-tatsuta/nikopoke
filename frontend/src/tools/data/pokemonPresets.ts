import { POKEMON_PRESETS as presets } from '../../lib/pokemonPresets';
import moves from '../../../public/data/moves.json';
import { moveData } from './catalog';
import type { EVs, Nature } from '../lib/stats';

type PokemonPreset = { itemId: string; nature?: Nature; evs: EVs; moveIds: string[] };
const byId = moves as Record<string, { name: string }>;
export const POKEMON_PRESETS: Record<string, PokemonPreset> = Object.fromEntries(
  Object.entries(presets).map(([id, preset]) => [id, {
    itemId: preset.itemId ?? '',
    nature: preset.nature as Nature | undefined,
    evs: preset.evs,
    moveIds: preset.moveNames.map(moveId => byId[moveId]?.name).filter((name): name is string =>
      Boolean(name && moveData.some(move => move.name === name && move.targets.includes(id)))
    ),
  }]),
);
