import { createClient } from '@supabase/supabase-js';
import type { BattleRecord } from './battleStats';
import type { DeckPokemon } from '../types/pokemon';

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL as string | undefined;
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY as string | undefined;

const supabase =
    supabaseUrl && supabaseAnonKey
        ? createClient(supabaseUrl, supabaseAnonKey)
        : null;

export type GlobalBattleTeamPokemon = {
    speciesId: string;
    moves: string[];
    ability: string;
};

export type GlobalBattleRecord = {
    id: string;
    created_at?: string;
    winner: 'host' | 'guest';
    host_team: GlobalBattleTeamPokemon[];
    guest_team: GlobalBattleTeamPokemon[];
};

function deckToGlobalTeam(deck: DeckPokemon[] | null | undefined): GlobalBattleTeamPokemon[] {
    if (!deck) return [];

    return deck.map((pokemon) => ({
        speciesId: pokemon.speciesId,
        moves: pokemon.moves.filter(Boolean),
        ability: pokemon.ability,
    }));
}

export async function uploadGlobalBattleRecord(args: {
    id: string;
    winner: string | null;
    hostDeck: DeckPokemon[] | null | undefined;
    guestDeck: DeckPokemon[] | null | undefined;
}) {
    if (!supabase) {
        console.warn('[globalBattleStats] Supabase env is not configured.');
        return;
    }

    if (args.winner !== 'host' && args.winner !== 'guest') {
        console.warn('[globalBattleStats] Skip upload: invalid winner', args.winner);
        return;
    }

    const record: GlobalBattleRecord = {
        id: args.id,
        winner: args.winner,
        host_team: deckToGlobalTeam(args.hostDeck),
        guest_team: deckToGlobalTeam(args.guestDeck),
    };

    const { error } = await supabase
        .from('battle_records')
        .upsert(record, { onConflict: 'id' });

    if (error) {
        // 同じidが既に入っている場合など。最初はログだけでOK。
        console.error('[globalBattleStats] Failed to upload battle record:', {
            message: error.message,
            code: error.code,
            details: error.details,
            hint: error.hint,
            full: error,
        });
    }
}

export async function loadGlobalBattleRecords(): Promise<GlobalBattleRecord[]> {
    if (!supabase) {
        return [];
    }

    const { data, error } = await supabase
        .from('battle_records')
        .select('id, created_at, winner, host_team, guest_team')
        .order('created_at', { ascending: false })
        .limit(500);

    if (error) {
        console.error('[globalBattleStats] Failed to load battle records:', error);
        return [];
    }

    return (data ?? []) as GlobalBattleRecord[];
}

export function globalRecordsToBattleRecords(records: GlobalBattleRecord[]): BattleRecord[] {
    return records.map((record) => ({
        id: record.id,
        createdAt: record.created_at ?? new Date().toISOString(),
        winner: record.winner,
        localPlayerId: 'host',
        opponentPlayerId: 'guest',
        mode: 'player',
        playerTeam: record.host_team.map((pokemon) => ({
            speciesId: pokemon.speciesId,
            moves: pokemon.moves,
        })),
        opponentTeam: record.guest_team.map((pokemon) => ({
            speciesId: pokemon.speciesId,
            moves: pokemon.moves,
        })),
    }));
}