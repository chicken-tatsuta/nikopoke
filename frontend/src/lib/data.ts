// Data loading utilities
import type { SpeciesData, MoveData, Learnset, ItemData } from '../types/pokemon';

let speciesCache: SpeciesData | null = null;
let movesCache: MoveData | null = null;
let learnsetsCache: Learnset | null = null;
let itemsCache: ItemData | null = null;
let moveIdMigrationsCache: Map<string, string> | null = null;

function dataUrl(path: string): string {
    return `${path}?v=${encodeURIComponent(__APP_VERSION__)}`;
}

export async function loadSpecies(): Promise<SpeciesData> {
    if (speciesCache) return speciesCache;

    const response = await fetch(dataUrl('/data/species.json'), { cache: 'no-cache' });
    const species = await response.json() as SpeciesData;
    let descriptions: Record<string, string> = {};

    try {
        const descriptionsResponse = await fetch(dataUrl('/data/speciesDescriptions.json'), { cache: 'no-cache' });
        descriptions = await descriptionsResponse.json();
    } catch {
        descriptions = {};
    }

    speciesCache = Object.fromEntries(
        Object.entries(species).map(([speciesId, mon]) => [
            speciesId,
            {
                ...mon,
                description: descriptions[speciesId]?.trim() || mon.description,
            },
        ]),
    );
    return speciesCache!;
}

export async function loadMoves(): Promise<MoveData> {
    if (movesCache) return movesCache;

    const response = await fetch(dataUrl('/data/moves.json'), { cache: 'no-cache' });
    movesCache = await response.json();
    return movesCache!;
}

export async function loadLearnsets(): Promise<Learnset> {
    if (learnsetsCache) return learnsetsCache;

    const response = await fetch(dataUrl('/data/learnsets.json'), { cache: 'no-cache' });
    learnsetsCache = await response.json();
    return learnsetsCache!;
}

export async function loadItems(): Promise<ItemData> {
    if (itemsCache) return itemsCache;

    const response = await fetch(dataUrl('/data/items.json'), { cache: 'no-cache' });
    itemsCache = await response.json();
    return itemsCache!;
}

export async function loadMoveIdMigrations(): Promise<Map<string, string>> {
    if (moveIdMigrationsCache) return moveIdMigrationsCache;

    try {
        const response = await fetch(dataUrl('/data/move_id_migration_report.json'), { cache: 'no-cache' });
        const migrations = await response.json() as { old_id?: string; new_id?: string }[];
        moveIdMigrationsCache = new Map(
            migrations
                .filter((entry) => entry.old_id && entry.new_id)
                .map((entry) => [entry.old_id!, entry.new_id!]),
        );
    } catch {
        moveIdMigrationsCache = new Map();
    }

    return moveIdMigrationsCache;
}

export function normalizeMoveName(name: string | undefined): string {
    return String(name || '')
        .replace(/[ \t\r\n\u3000]+/g, '')
        .trim();
}

export function canonicalizeMoveId(
    moveId: string,
    moves: MoveData,
    moveIdMigrations: Map<string, string>,
): string {
    const migratedMoveId = moveIdMigrations.get(moveId);
    if (!migratedMoveId || !moves[migratedMoveId]) {
        return moveId;
    }

    if (!moves[moveId]) {
        return migratedMoveId;
    }

    const currentName = normalizeMoveName(moves[moveId]?.name);
    const migratedName = normalizeMoveName(moves[migratedMoveId]?.name);
    return currentName && currentName === migratedName ? migratedMoveId : moveId;
}

export function canonicalizeMoveIds(
    moveIds: string[],
    moves: MoveData,
    moveIdMigrations: Map<string, string>,
): string[] {
    return moveIds
        .map((moveId) => canonicalizeMoveId(moveId, moves, moveIdMigrations))
        .filter((moveId, index, self) => self.indexOf(moveId) === index);
}

function canonicalizeLearnsets(
    learnsets: Learnset,
    moves: MoveData,
    moveIdMigrations: Map<string, string>,
): Learnset {
    return Object.fromEntries(
        Object.entries(learnsets).map(([speciesId, moveIds]) => [
            speciesId,
            canonicalizeMoveIds(moveIds, moves, moveIdMigrations),
        ]),
    );
}

function buildSafeMoveIdMigrations(
    moveIdMigrations: Map<string, string>,
    moves: MoveData,
    learnsets: Learnset,
): Map<string, string> {
    const learnsetUseCounts = new Map<string, number>();
    for (const moveIds of Object.values(learnsets)) {
        for (const moveId of moveIds) {
            learnsetUseCounts.set(moveId, (learnsetUseCounts.get(moveId) ?? 0) + 1);
        }
    }

    const safeMigrations = new Map<string, string>();
    for (const [oldId, newId] of moveIdMigrations.entries()) {
        if (!moves[newId]) {
            continue;
        }

        if (!moves[oldId]) {
            safeMigrations.set(oldId, newId);
            continue;
        }

        const oldName = normalizeMoveName(moves[oldId]?.name);
        const newName = normalizeMoveName(moves[newId]?.name);
        if (oldName && oldName === newName && !learnsetUseCounts.has(oldId)) {
            safeMigrations.set(oldId, newId);
        }
    }

    return safeMigrations;
}

export async function loadAllData() {
    const [species, moves, learnsets, items, rawMoveIdMigrations] = await Promise.all([
        loadSpecies(),
        loadMoves(),
        loadLearnsets(),
        loadItems(),
        loadMoveIdMigrations(),
    ]);
    const moveIdMigrations = buildSafeMoveIdMigrations(rawMoveIdMigrations, moves, learnsets);
    return {
        species,
        moves,
        items,
        learnsets: canonicalizeLearnsets(learnsets, moves, moveIdMigrations),
        moveIdMigrations,
    };
}

// Type color mapping - Muted sophisticated palette
export const TYPE_COLORS: { [key: string]: string } = {
    normal: '#8b8d94',
    fire: '#b45c40',
    water: '#4a7c9b',
    electric: '#c9a94d',
    grass: '#5a8a6a',
    ice: '#6b9fa8',
    fighting: '#8c5a5a',
    poison: '#7a5c82',
    ground: '#a08a6a',
    flying: '#7a8ab0',
    psychic: '#a86a85',
    bug: '#7a8a40',
    rock: '#8a7a5a',
    ghost: '#5a5a7a',
    dragon: '#5a5a9a',
    dark: '#5a4a4a',
    steel: '#7a8a8a',
    fairy: '#a87a8a',
};

export function getTypeColor(type: string): string {
    return TYPE_COLORS[type.toLowerCase()] || '#6b7280';
}
