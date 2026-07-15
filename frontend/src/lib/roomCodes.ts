import { supabase } from './supabase';

const ROOM_CODE_TTL_MINUTES = 5;

/** Register a code → peerID mapping. Throws if code is already taken. */
export async function registerRoomCode(code: string, peerId: string): Promise<void> {
    if (!supabase) {
        console.warn('[roomCodes] Supabase not configured, room code registration skipped (code:', code, 'peer:', peerId, ')');
        return;
    }

    const { error } = await supabase.rpc('register_room_code', {
        p_code: code,
        p_peer_id: peerId,
    });

    if (error) {
        console.error('[roomCodes] register failed:', error.message, { code, peerId });
        if (
            error.message?.includes('already in use') ||
            error.message?.includes('duplicate') ||
            error.message?.includes('already exists')
        ) {
            throw new Error(`ルームコード "${code}" は既に使われています。別のコードを試してください。`);
        }
        throw new Error(`ルーム登録に失敗しました: ${error.message}`);
    }

    console.log('[roomCodes] room code registered:', code, '->', peerId);
}

/** Look up a peer ID by room code. Returns null if code is not found or expired. */
export async function lookupRoomCode(code: string): Promise<string | null> {
    if (!supabase) {
        console.warn('[roomCodes] Supabase not configured, lookup skipped (code:', code, ')');
        return null;
    }

    const { data, error } = await supabase.rpc('lookup_room_code', {
        p_code: code,
    });

    if (error) {
        console.warn('[roomCodes] lookup failed:', error.message, { code });
        return null;
    }

    return (data as string | null) ?? null;
}

/** Clean up expired room codes. Returns number of deleted rows. */
export async function cleanupExpiredRoomCodes(): Promise<number> {
    if (!supabase) {
        return 0;
    }

    const { data, error } = await supabase.rpc('cleanup_expired_room_codes');

    if (error) {
        console.warn('[roomCodes] cleanup failed:', error.message);
        return 0;
    }

    return (data as number) ?? 0;
}

export { ROOM_CODE_TTL_MINUTES };
