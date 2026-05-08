// Engine wrapper for browser WASM integration
import init, {
    createBattleState as wasmCreateBattleState,
    createCreature as wasmCreateCreature,
    stepBattle as wasmStepBattle,
    getBestMoveMCTS as wasmGetBestMoveMCTS,
    getBestMoveMinimax as wasmGetBestMoveMinimax,
    isBattleOver as wasmIsBattleOver,
    replaceFaintedPokemon as wasmReplaceFaintedPokemon,
} from './engine-rust/engine_rust.js';

import type { DeckPokemon, EVStats } from '../types/pokemon';
import { loadAllData } from './data';
import { normalizeEvs } from './evs';

// WASM initialization state
let wasmInitialized = false;
let wasmInitPromise: Promise<void> | null = null;
const moveCompatibilityCache = new Map<string, boolean>();

export async function initEngine(): Promise<void> {
    if (wasmInitialized) return;
    if (wasmInitPromise) return wasmInitPromise;

    wasmInitPromise = (async () => {
        await init();
        wasmInitialized = true;
    })();

    return wasmInitPromise;
}

// Types matching WASM wire format
export interface CreatureStateWire {
    id: string;
    speciesId: string;
    name: string;
    level: number;
    types: string[];
    moves: string[];
    ability: string | null;
    item: string | null;
    evs: EVStats;
    hp: number;
    maxHp: number;
    stages: { atk: number; def: number; spa: number; spd: number; spe: number; accuracy: number; evasion: number };
    statuses: { id: string; remainingTurns: number | null }[];
    movePp: { [moveId: string]: number };
    attack: number;
    defense: number;
    spAttack: number;
    spDefense: number;
    speed: number;
    weightKg?: number;
}

export interface PlayerStateWire {
    id: string;
    name: string;
    team: CreatureStateWire[];
    activeSlot: number;
}

export interface FieldStateWire {
    global: { id: string; remainingTurns: number | null }[];
    sides: { [playerId: string]: { id: string; remainingTurns: number | null }[] };
}

export interface BattleStateWire {
    players: PlayerStateWire[];
    field: FieldStateWire;
    turn: number;
    log: string[];
}

export interface ActionWire {
    type: 'move' | 'switch';
    playerId: string;
    moveId?: string;
    targetId?: string;
    slot?: number;
}

const RESET_STAGES: CreatureStateWire['stages'] = {
    atk: 0,
    def: 0,
    spa: 0,
    spd: 0,
    spe: 0,
    accuracy: 0,
    evasion: 0,
};

const NON_VOLATILE_STATUS_IDS = new Set([
    'burn',
    'poison',
    'toxic',
    'badly_poisoned',
    'paralysis',
    'freeze',
    'sleep',
]);

function hasPendingSwitch(creature: CreatureStateWire): boolean {
    return creature.statuses.some((status) => status.id === 'pending_switch');
}

export function needsForcedSwitch(state: BattleStateWire, playerId: string): boolean {
    const player = state.players.find((candidate) => candidate.id === playerId);
    if (!player) {
        return false;
    }
    const active = player.team[player.activeSlot];
    return active.hp <= 0 || hasPendingSwitch(active);
}

export function getFirstAvailableSwitchSlot(state: BattleStateWire, playerId: string): number | null {
    const player = state.players.find((candidate) => candidate.id === playerId);
    if (!player) {
        return null;
    }
    const slot = player.team.findIndex((creature, index) => index !== player.activeSlot && creature.hp > 0);
    return slot >= 0 ? slot : null;
}

function replaceFaintedPokemonLocally(
    state: BattleStateWire,
    playerId: string,
    slot: number,
): BattleStateWire {
    const nextState = structuredClone(state) as BattleStateWire;
    const player = nextState.players.find((candidate) => candidate.id === playerId);
    if (!player) {
        nextState.log.push(`unknown player ${playerId} cannot replace pokemon.`);
        return nextState;
    }

    const outgoing = player.team[player.activeSlot];
    const incoming = player.team[slot];
    if (!incoming || slot === player.activeSlot || incoming.hp <= 0) {
        nextState.log.push(`${player.name}は 交代先を選べない！`);
        return nextState;
    }

    if (outgoing.hp > 0 && !hasPendingSwitch(outgoing)) {
        nextState.log.push(`${outgoing.name}は まだ戦える！`);
        return nextState;
    }

    outgoing.stages = { ...RESET_STAGES };
    outgoing.statuses = outgoing.statuses.filter((status) => NON_VOLATILE_STATUS_IDS.has(status.id));
    player.activeSlot = slot;
    incoming.statuses = incoming.statuses.filter((status) => status.id !== 'pending_switch');
    nextState.log.push(`${player.name}は ${incoming.name}を 繰り出した！`);
    return nextState;
}

export function replaceFaintedPokemon(
    state: BattleStateWire,
    playerId: string,
    slot: number,
): BattleStateWire {
    if (wasmInitialized) {
        return wasmReplaceFaintedPokemon(state, playerId, slot) as BattleStateWire;
    }
    return replaceFaintedPokemonLocally(state, playerId, slot);
}

function normalizeMoveName(name: string | undefined): string {
    return String(name || '')
        .replace(/[ \t\r\n\u3000]+/g, '')
        .trim();
}

function buildSameNameMoveIds(
    moves: Record<string, { name?: string }>,
): Map<string, string[]> {
    const byName = new Map<string, string[]>();
    for (const [moveId, move] of Object.entries(moves)) {
        const normalizedName = normalizeMoveName(move?.name);
        if (!normalizedName) {
            continue;
        }
        const existing = byName.get(normalizedName) ?? [];
        existing.push(moveId);
        byName.set(normalizedName, existing);
    }
    return byName;
}

function canUseMoveWithCurrentWasm(speciesId: string, moveId: string): boolean {
    const cacheKey = `${speciesId}:${moveId}`;
    if (moveCompatibilityCache.has(cacheKey)) {
        return moveCompatibilityCache.get(cacheKey)!;
    }
    let usable = false;
    try {
        const creature = wasmCreateCreature(speciesId, { moves: [moveId] }) as CreatureStateWire;
        usable = Array.isArray(creature.moves) && creature.moves.includes(moveId);
    } catch {
        usable = false;
    }
    moveCompatibilityCache.set(cacheKey, usable);
    return usable;
}

function resolveCompatibleMoveId(
    speciesId: string,
    moveId: string,
    moves: Record<string, { name?: string }>,
    sameNameMoveIds: Map<string, string[]>,
): string | null {
    if (canUseMoveWithCurrentWasm(speciesId, moveId)) {
        return moveId;
    }

    // Only fallback to move IDs that share the exact same move name.
    // This prevents conversions like "bulk_up -> superpower" that change behavior.
    const moveName = normalizeMoveName(moves[moveId]?.name);
    if (!moveName) {
        return null;
    }
    const candidates = sameNameMoveIds.get(moveName) ?? [];
    for (const candidateId of candidates) {
        if (candidateId === moveId) {
            continue;
        }
        if (canUseMoveWithCurrentWasm(speciesId, candidateId)) {
            return candidateId;
        }
    }
    return null;
}

function normalizeDeckPokemon(
    pokemon: DeckPokemon,
    learnsets: Record<string, string[]>,
    moves: Record<string, { pp: number; name?: string }>,
    sameNameMoveIds: Map<string, string[]>,
): DeckPokemon {
    const learnableMoves = learnsets[pokemon.speciesId] ?? [];

    const selectedMoves = pokemon.moves
        .filter((moveId, index, self) => self.indexOf(moveId) === index)
        .map((moveId) => {
            if (moves[moveId]) {
                return moveId;
            }

            return resolveCompatibleMoveId(pokemon.speciesId, moveId, moves, sameNameMoveIds);
        })
        .filter((moveId): moveId is string => Boolean(moveId))
        .filter((moveId, index, self) => self.indexOf(moveId) === index)
        .slice(0, 4);

    for (const moveId of learnableMoves) {
        if (selectedMoves.length >= 4) {
            break;
        }

        if (!moves[moveId]) {
            continue;
        }

        const compatibleMoveId = resolveCompatibleMoveId(pokemon.speciesId, moveId, moves, sameNameMoveIds);

        if (compatibleMoveId && !selectedMoves.includes(compatibleMoveId)) {
            selectedMoves.push(compatibleMoveId);
        }
    }

    return {
        ...pokemon,
        evs: normalizeEvs(pokemon.evs),
        moves: selectedMoves,
    };
}

// Initialize and create battle state
export async function createBattleState(playerDecks: {
    [playerId: string]: { team: DeckPokemon[] }
}): Promise<BattleStateWire> {
    await initEngine();
    const { moves, learnsets } = await loadAllData();
    const sameNameMoveIds = buildSameNameMoveIds(moves);

    // Create creatures for each player
    const players: PlayerStateWire[] = [];

    for (const [playerId, playerData] of Object.entries(playerDecks)) {
        const team: CreatureStateWire[] = [];

        for (const pokemon of playerData.team) {
            const normalizedPokemon = normalizeDeckPokemon(pokemon, learnsets, moves, sameNameMoveIds);
            const creature = wasmCreateCreature(normalizedPokemon.speciesId, {
                moves: normalizedPokemon.moves,
                ability: normalizedPokemon.ability,
                evs: normalizedPokemon.evs,
            });
            team.push(creature);
        }

        players.push({
            id: playerId,
            name: playerId,
            team,
            activeSlot: 0,
        });
    }

    return wasmCreateBattleState(players);
}

export async function stepBattle(
    state: BattleStateWire,
    actions: ActionWire[]
): Promise<BattleStateWire> {
    await initEngine();
    return wasmStepBattle(state, actions, { recordHistory: false });
}

export async function getBestMoveMCTS(
    state: BattleStateWire,
    playerId: string,
    iterations: number = 100
): Promise<ActionWire | null> {
    await initEngine();
    return wasmGetBestMoveMCTS(state, playerId, iterations);
}

export async function getBestMoveMinimax(
    state: BattleStateWire,
    playerId: string,
    depth: number = 3
): Promise<ActionWire | null> {
    await initEngine();
    return wasmGetBestMoveMinimax(state, playerId, depth);
}

export async function isBattleOver(state: BattleStateWire): Promise<boolean> {
    await initEngine();
    return wasmIsBattleOver(state);
}

// Helper to check winner
export function getWinner(state: BattleStateWire): string | null {
    for (const player of state.players) {
        const allFainted = player.team.every(c => c.hp <= 0);
        if (allFainted) {
            // Return the OTHER player's ID as winner
            const winner = state.players.find(p => p.id !== player.id);
            return winner?.id || null;
        }
    }
    return null;
}
