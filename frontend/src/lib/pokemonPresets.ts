import type { EVStats, MoveData } from "../types/pokemon";
import { canonicalizeMoveId } from "./data";

export type PokemonPreset = {
  itemId?: string;
  abilityId?: string;
  nature?: {
    name: string;
    increased: keyof EVStats;
    decreased: keyof EVStats;
  };
  evs: EVStats;
  moveNames: string[];
};

const createEvs = (evs: Partial<EVStats>): EVStats => ({
  hp: 0,
  atk: 0,
  def: 0,
  spa: 0,
  spd: 0,
  spe: 0,
  ...evs,
});

const MODEST = {
  name: "ひかえめ",
  increased: "spa",
  decreased: "atk",
} as const;
const TIMID = {
  name: "おくびょう",
  increased: "spe",
  decreased: "atk",
} as const;
const ADAMANT = {
  name: "いじっぱり",
  increased: "atk",
  decreased: "spa",
} as const;
const JOLLY = { name: "ようき", increased: "spe", decreased: "spa" } as const;

export const POKEMON_PRESETS: Record<string, PokemonPreset> = {
  eiraku: {
    itemId: "choiceSpecs",
    nature: MODEST,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["hydro_pump", "boomburst", "aura_sphere", "torch_song"],
  },
  tatuta: {
    itemId: "choiceScarf",
    nature: TIMID,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["leaf_storm", "draco_meteor", "torch_song", "spore"],
  },
  morimitu: {
    itemId: "lifeOrb",
    nature: TIMID,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["overheat", "air_slash", "thunderbolt", "moonblast"],
  },
  takaho: {
    itemId: "assaultVest",
    nature: ADAMANT,
    evs: createEvs({ hp: 32, atk: 32, spd: 1 }),
    moveNames: ["horn_leech", "u_turn", "detect", "accelerock"],
  },
  ume: {
    itemId: "choiceSpecs",
    nature: MODEST,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["will_o_wisp", "thunderbolt", "volt_switch", "hex"],
  },
  machida: {
    itemId: "choiceBand",
    nature: JOLLY,
    evs: createEvs({ atk: 32, spe: 32, hp: 1 }),
    moveNames: ["earthquake", "liquidation", "flip_turn", "pyro_ball"],
  },
  touma: {
    itemId: "choiceScarf",
    nature: JOLLY,
    evs: createEvs({ atk: 32, spe: 32, hp: 1 }),
    moveNames: ["close_combat", "knock_off", "swords_dance", "sucker_punch"],
  },
  morimori: {
    itemId: "assaultVest",
    nature: ADAMANT,
    evs: createEvs({ hp: 32, atk: 1, def: 32 }),
    moveNames: ["earthquake", "rest", "body_press", "iron_defense"],
  },
  ayuma: {
    itemId: "lifeOrb",
    nature: TIMID,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["nasty_plot", "dark_pulse", "flash_cannon", "aura_sphere"],
  },
  buchii: {
    itemId: "assaultVest",
    nature: MODEST,
    evs: createEvs({ hp: 32, def: 32, spd: 1 }),
    moveNames: ["moonblast", "bug_buzz", "psyshock", "quiver_dance"],
  },
  tomoki: {
    itemId: "choiceSpecs",
    nature: TIMID,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["ice_beam", "hurricane", "thunderbolt", "flamethrower"],
  },
  haruta: {
    itemId: "choiceBand",
    nature: JOLLY,
    evs: createEvs({ atk: 32, spe: 32, hp: 1 }),
    moveNames: ["extreme_speed", "play_rough", "pyro_ball", "protect"],
  },
  macchan: {
    itemId: "choiceBand",
    nature: JOLLY,
    evs: createEvs({ atk: 32, spe: 32, hp: 1 }),
    moveNames: ["close_combat", "bullet_punch", "meteor_mash", "dragon_dance"],
  },
  zosueda: {
    abilityId: "steely_spirit",
    nature: ADAMANT,
    evs: createEvs({ hp: 32, atk: 32, def: 1 }),
    moveNames: ["bullet_punch", "meteor_mash", "iron_defense", "flare_blitz"],
  },
  shiraishi: {
    abilityId: "disguise",
    nature: MODEST,
    evs: createEvs({ hp: 32, spa: 32, spe: 1 }),
    moveNames: ["scorching_sands", "moonblast", "freeze_dry", "psychic_noise"],
  },
  otyamichi: {
    abilityId: "hospitality",
    nature: MODEST,
    evs: createEvs({ hp: 32, spa: 32, spd: 1 }),
    moveNames: ["shaka_shaka_ho", "leech_seed", "protect", "chilling_water"],
  },
  toumac: {
    abilityId: "early_bird",
    nature: JOLLY,
    evs: createEvs({ atk: 32, spe: 32, hp: 1 }),
    moveNames: ["rage_fist", "knock_off", "close_combat", "rest"],
  },
  michii: {
    itemId: "choiceSpecs",
    nature: TIMID,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["psychic", "dark_pulse", "thunderbolt", "nasty_plot"],
  },
  nisiki: {
    itemId: "focusSash",
    nature: TIMID,
    evs: createEvs({ spa: 32, spe: 32, def: 1 }),
    moveNames: ["moongeist_beam", "psychic", "freeze_dry", "will_o_wisp"],
  },
  sena: {
    itemId: "choiceBand",
    nature: ADAMANT,
    evs: createEvs({ hp: 32, atk: 32, spd: 1 }),
    moveNames: [
      "first_impression",
      "volt_tackle",
      "iron_head",
      "headlong_rush",
    ],
  },
  ikkun: {
    itemId: "assaultVest",
    nature: ADAMANT,
    evs: createEvs({ hp: 32, def: 32, spd: 1 }),
    moveNames: ["accelerock", "substitute", "toxic", "protect"],
  },
  futo: {
    itemId: "choiceScarf",
    nature: JOLLY,
    evs: createEvs({ atk: 32, spe: 32, hp: 1 }),
    moveNames: ["sacred_fire", "flare_blitz", "megahorn", "u_turn"],
  },
  makocchan: {
    itemId: "lifeOrb",
    nature: MODEST,
    evs: createEvs({ hp: 32, spa: 32, spd: 1 }),
    moveNames: ["moonblast", "psychic", "calm_mind", "mystical_fire"],
  },
  reosan: {
    itemId: "choiceBand",
    nature: ADAMANT,
    evs: createEvs({ hp: 32, atk: 32, spd: 1 }),
    moveNames: ["poltergeist", "megahorn", "first_impression", "wish"],
  },
};

export function getPokemonPreset(speciesId: string): PokemonPreset | undefined {
  return POKEMON_PRESETS[speciesId];
}

export function resolvePresetMoveIds(
  preset: PokemonPreset | undefined,
  moves: MoveData,
  fallbackMoveIds: string[],
  moveIdMigrations: Map<string, string> = new Map(),
): string[] {
  const fallbackValidMoveIds = fallbackMoveIds.filter(
    (moveId) => moves[moveId],
  );

  if (!preset) {
    return fallbackValidMoveIds.slice(0, 4);
  }

  const moveIdByName = new Map<string, string>();

  for (const [moveId, move] of Object.entries(moves)) {
    moveIdByName.set(move.name, moveId);
  }

  const presetMoveIds = preset.moveNames
    .map((moveName) => {
      if (moves[moveName]) {
        return canonicalizeMoveId(moveName, moves, moveIdMigrations);
      }

      const moveId = moveIdByName.get(moveName);
      return moveId ? canonicalizeMoveId(moveId, moves, moveIdMigrations) : undefined;
    })
    .filter((moveId): moveId is string => Boolean(moveId))
    .filter((moveId) => moves[moveId]);

  const uniqueMoveIds = Array.from(new Set(presetMoveIds));

  for (const moveId of fallbackValidMoveIds) {
    if (uniqueMoveIds.length >= 4) break;
    if (!uniqueMoveIds.includes(moveId)) {
      uniqueMoveIds.push(moveId);
    }
  }

  return uniqueMoveIds.slice(0, 4);
}
