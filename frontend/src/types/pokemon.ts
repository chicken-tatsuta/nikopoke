// Type definitions for Nikipoke

export interface BaseStats {
    hp: number;
    atk: number;
    def: number;
    spa: number;
    spd: number;
    spe: number;
}

export interface Species {
    id: string;
    name: string;
    description?: string;
    type: string[];
    baseStats: BaseStats;
    abilities: string[];
}

export interface Move {
    id: string;
    name: string;
    type: string;
    power: number | null;
    pp: number;
    accuracy: number | null;
    category: "physical" | "special" | "status";
    description: string;
    priority?: number;
}

// Effort Values (EVs) - max 32 per stat, 66 total
export interface EVStats {
    hp: number;
    atk: number;
    def: number;
    spa: number;
    spd: number;
    spe: number;
}

export interface Learnset {
    [speciesId: string]: string[];
}

export interface SpeciesData {
    [id: string]: Species;
}

export interface MoveData {
    [id: string]: Move;
}

export interface Item {
    id: string;
    name: string;
    kind: "boost" | "resist" | "attack" | "defense" | "support";
    type?: string;
}

export interface ItemData {
    [id: string]: Item;
}

// Battle State Types
export interface BattleCreature {
    id: string;
    speciesId: string;
    name: string;
    level: number;
    currentHp: number;
    maxHp: number;
    moves: string[];
    ability: string;
    item?: string | null;
    consumedItem?: string | null;
    status?: string;
    stats: BaseStats;
}

export interface BattleAction {
    type: "move" | "switch";
    moveId?: string;
    switchTo?: number;
}

export interface Player {
    id: string;
    team: BattleCreature[];
    activeIndex: number;
}

export interface BattleState {
    players: { [playerId: string]: Player };
    logs: string[];
    turn: number;
}

// Deck for battle
export interface Deck {
    name: string;
    pokemon: DeckPokemon[];
}

export interface DeckPokemon {
    speciesId: string;
    moves: string[];
    ability: string;
    item?: string | null;
    evs?: EVStats;
}
