import type { PokemonUsageStats } from './battleStats';
import type { DeckPokemon } from '../types/pokemon';
import { supabase } from './supabase';

export type GlobalBattleTeamPokemon = {
    speciesId: string;
    moves: string[];
    ability: string;
};


export type GlobalBattleRecord = {
    id: string;
    created_at?: string;
    winner: 'host' | 'guest';
    host_user_id?: string | null;
    guest_user_id?: string | null;
    host_team: GlobalBattleTeamPokemon[];
    guest_team: GlobalBattleTeamPokemon[];

};

    type PokemonUsageStatsRow = {
    species_id: string;
    used_count: number;
    win_count: number;
    loss_count: number;
};

type PokemonMoveUsageStatsRow = {
    species_id: string;
    move_id: string;
    used_count: number;

};

function deckToGlobalTeam(deck: DeckPokemon[] | null | undefined): GlobalBattleTeamPokemon[] {
    if (!deck) return [];

    return deck.map((pokemon) => ({
        speciesId: pokemon.speciesId,
        moves: pokemon.moves.filter(Boolean),
        ability: pokemon.ability,
    }));
}

export function createBattleStatsId(): string {
    return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

export async function uploadGlobalBattleRecord(args: {
  id: string;
  winner: string | null;
  hostDeck?: DeckPokemon[] | null;
  guestDeck?: DeckPokemon[] | null;
  host_user_id?: string | null;
  guest_user_id?: string | null;
  mode?: 'ai' | 'player';
}) {
    if (!supabase) {
        console.warn('[globalBattleStats] Supabase env is not configured.');
        return;
    }

    if (args.winner !== 'player' && args.winner !== 'opponent') {
        console.warn('[globalBattleStats] Skip upload: invalid winner', args.winner);
        return;
    }

const { error } = await supabase.rpc('record_battle_result', {
  p_battle_id: args.id,
  p_mode: args.mode,
  p_winner_side: args.winner,
  p_player_team: deckToGlobalTeam(args.hostDeck),
  p_opponent_team: deckToGlobalTeam(args.guestDeck),
  p_player_user_id: args.host_user_id ?? null,
  p_opponent_user_id: args.guest_user_id ?? null,
});

    if (error) {
        console.error('[globalBattleStats] Failed to upload battle record:', {
            message: error.message,
            code: error.code,
            details: error.details,
            hint: error.hint,
            full: error,
        });
    }
}

export async function loadGlobalPokemonUsageStats(): Promise<Record<string, PokemonUsageStats>> {
    if (!supabase) {
        return {};
    }

    const [{ data: usageRows, error: usageError }, { data: moveRows, error: moveError }] = await Promise.all([
        supabase
            .from('pokemon_usage_stats')
            .select('species_id, used_count, win_count, loss_count')
            .order('used_count', { ascending: false }),
        supabase
            .from('pokemon_move_usage_stats')
            .select('species_id, move_id, used_count'),
    ]);

    if (usageError) {
        console.error('[globalBattleStats] Failed to load pokemon usage summary:', usageError);
        return {};
    }

    if (moveError) {
        console.warn('[globalBattleStats] Failed to load move usage stats. Falling back to usage summary only.', moveError);
    }

    const movesBySpecies = new Map<string, PokemonMoveUsageStatsRow[]>();

    for (const row of ((moveRows ?? []) as PokemonMoveUsageStatsRow[])) {
        const current = movesBySpecies.get(row.species_id) ?? [];
        current.push(row);
        movesBySpecies.set(row.species_id, current);
    }

    return Object.fromEntries(
        ((usageRows ?? []) as PokemonUsageStatsRow[]).map((row) => {
            const used = Number(row.used_count ?? 0);
            const wins = Number(row.win_count ?? 0);
            const losses = Number(row.loss_count ?? 0);
            const moves = (movesBySpecies.get(row.species_id) ?? [])
                .map((moveRow) => ({
                    name: moveRow.move_id,
                    rate: used > 0 ? (Number(moveRow.used_count ?? 0) / used) * 100 : 0,
                }))
                .sort((left, right) => right.rate - left.rate)
                .slice(0, 8);

            return [
                row.species_id,
                {
                    speciesId: row.species_id,
                    used,
                    wins,
                    losses,
                    winRate: used > 0 ? (wins / used) * 100 : 0,
                    moves,
                },
            ];
        }),
    );
}
