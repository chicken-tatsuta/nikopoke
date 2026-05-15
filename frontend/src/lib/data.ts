// Data loading utilities
import type { SpeciesData, MoveData, Learnset } from '../types/pokemon';

let speciesCache: SpeciesData | null = null;
let movesCache: MoveData | null = null;
let learnsetsCache: Learnset | null = null;

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

export async function loadAllData() {
    const [species, moves, learnsets] = await Promise.all([
        loadSpecies(),
        loadMoves(),
        loadLearnsets()
    ]);
    return { species, moves, learnsets };
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
