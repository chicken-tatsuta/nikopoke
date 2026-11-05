import registry from '../../../engine-rust/data/kamishinka.json';
import type { BaseStats, Species, SpeciesData } from '../types/pokemon';
import type { BattleStateWire, CreatureStateWire, PlayerStateWire } from './engine';
import { getPokemonPortraitSrc } from './pokemonImages';

export interface KamishinkaForm {
    baseSpeciesId: string;
    id: string;
    name: string;
    baseStats: BaseStats;
    types: string[];
    ability: string;
    /** Optional public image URL; null reuses the normal portrait. */
    image: string | null;
}
export const kamishinkaForms: KamishinkaForm[] = registry;

export function availableKamishinkaForms(player: PlayerStateWire): KamishinkaForm[] {
    const active = player.team[player.activeSlot];
    if (!active || player.kamishinkaUsed || active.kamishinka || active.hp <= 0
        || active.item !== 'kamidama' || active.statuses.some((s) => s.id === 'pending_switch')) return [];
    return kamishinkaForms.filter((f) => f.baseSpeciesId === active.speciesId);
}

export function getBattlePortrait(creature: CreatureStateWire): string {
    const form = kamishinkaForms.find((f) => f.id === creature.kamishinka?.formId);
    return form?.image ?? getPokemonPortraitSrc(form?.baseSpeciesId ?? creature.speciesId, creature.kamishinka?.originalName ?? creature.name);
}

/** Playback only: never copy final HP, PP, statuses or stages ahead of the move animation. */
export function applyVisualForm(creature: CreatureStateWire, formId: string): void {
    const form = kamishinkaForms.find((f) => f.id === formId && f.baseSpeciesId === creature.speciesId);
    if (!form || creature.kamishinka) return;
    creature.kamishinka = {
        formId, originalName: creature.name, originalAbility: creature.ability,
        originalTypes: [...creature.types],
        originalStats: [creature.attack, creature.defense, creature.spAttack, creature.spDefense, creature.speed],
    };
    creature.name = form.name;
    creature.types = [...form.types];
    creature.ability = form.ability;
    const stat = (key: 'atk' | 'def' | 'spa' | 'spd' | 'spe') =>
        Math.floor((form.baseStats[key] * 2 + 31) * creature.level / 100) + 5 + creature.evs[key];
    creature.attack = stat('atk'); creature.defense = stat('def');
    creature.spAttack = stat('spa'); creature.spDefense = stat('spd'); creature.speed = stat('spe');
}

export function resetKamishinkaState(state: BattleStateWire): BattleStateWire {
    const next = structuredClone(state);
    for (const player of next.players) {
        player.kamishinkaUsed = false;
        for (const creature of player.team) {
            const original = creature.kamishinka;
            if (!original) continue;
            creature.name = original.originalName;
            creature.types = original.originalTypes;
            creature.ability = original.originalAbility;
            [creature.attack, creature.defense, creature.spAttack, creature.spDefense, creature.speed] = original.originalStats;
            creature.kamishinka = null;
        }
    }
    return next;
}

/** Display form stats without adding evolved entries to the normal selection/Pokédex. */
export function getBattleSpecies(creature: CreatureStateWire, species: SpeciesData): Species | undefined {
    const base = species[creature.speciesId];
    if (!base) return undefined;
    const form = kamishinkaForms.find((f) => f.id === creature.kamishinka?.formId);
    return form ? { ...base, name: creature.name, type: creature.types, baseStats: form.baseStats,
        abilities: creature.ability ? [creature.ability] : [] } : base;
}
