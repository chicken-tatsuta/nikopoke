/* eslint-disable react-refresh/only-export-components */
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import type { Session, User } from '@supabase/supabase-js';
import { supabase } from '../lib/supabase';
import type { DeckPokemon } from '../types/pokemon';

export type SavedDeck = {
    name: string;
    deck: DeckPokemon[];
};

export type Profile = {
    id: string;
    username: string;
    rating: number;
    win_count: number;
    loss_count: number;
    is_admin: boolean;
    current_deck: DeckPokemon[] | null;
    saved_decks: SavedDeck[];
};

type ProfileUpdates = Partial<Pick<Profile, 'username' | 'current_deck' | 'saved_decks'>>;

type AuthContextValue = {
    session: Session | null;
    user: User | null;
    profile: Profile | null;
    loading: boolean;
    signIn: (email: string, password: string) => Promise<void>;
    signUp: (email: string, password: string, username: string) => Promise<void>;
    signOut: () => Promise<void>;
    updateProfile: (updates: ProfileUpdates) => Promise<void>;
    refreshProfile: () => Promise<void>;
};

export const USERNAME_MAX_LENGTH = 16;

const AuthContext = createContext<AuthContextValue | null>(null);

function normalizeUsername(username: string): string {
    return username.trim();
}

function assertValidUsername(username: string) {
    const normalized = normalizeUsername(username);
    if (!normalized) {
        throw new Error('ユーザー名を入力してください。');
    }
    if (normalized.length > USERNAME_MAX_LENGTH) {
        throw new Error(`ユーザー名は${USERNAME_MAX_LENGTH}文字以内にしてください。`);
    }
    return normalized;
}

function normalizeProfile(row: Profile): Profile {
    return {
        ...row,
        rating: Number(row.rating ?? 1500),
        is_admin: Boolean(row.is_admin),
        current_deck: Array.isArray(row.current_deck) ? row.current_deck : null,
        saved_decks: Array.isArray(row.saved_decks) ? row.saved_decks : [],
    };
}

async function fetchProfile(userId: string): Promise<Profile | null> {
    if (!supabase) return null;

    const { data, error } = await supabase
        .from('profiles')
        .select('*')
        .eq('id', userId)
        .maybeSingle();

    if (error) {
        console.error('[auth] Failed to load profile:', error);
        return null;
    }

    return data ? normalizeProfile(data as Profile) : null;
}

export function AuthProvider({ children }: { children: ReactNode }) {
    const [session, setSession] = useState<Session | null>(null);
    const [profile, setProfile] = useState<Profile | null>(null);
    const [loading, setLoading] = useState(() => Boolean(supabase));

    const loadProfile = useCallback(async (nextSession: Session | null) => {
        if (!nextSession?.user || !supabase) {
            setProfile(null);
            return;
        }

        setProfile(await fetchProfile(nextSession.user.id));
    }, []);

    useEffect(() => {
        if (!supabase) {
            return;
        }

        let active = true;

        supabase.auth.getSession().then(async ({ data }) => {
            if (!active) return;
            setSession(data.session);
            await loadProfile(data.session);
            if (active) setLoading(false);
        });

        const { data: listener } = supabase.auth.onAuthStateChange((_event, nextSession) => {
            setSession(nextSession);
            void loadProfile(nextSession);
        });

        return () => {
            active = false;
            listener.subscription.unsubscribe();
        };
    }, [loadProfile]);

    const signIn = useCallback(async (email: string, password: string) => {
        if (!supabase) throw new Error('Supabase が設定されていません。');

        const { error } = await supabase.auth.signInWithPassword({ email, password });
        if (error) throw error;
    }, []);

    const signUp = useCallback(async (email: string, password: string, username: string) => {
        if (!supabase) throw new Error('Supabase が設定されていません。');
        const validUsername = assertValidUsername(username);

        const { data, error } = await supabase.auth.signUp({
            email,
            password,
            options: {
                data: { username: validUsername },
            },
        });
        if (error) throw error;

        if (data.user && data.session) {
            const { error: profileError } = await supabase
                .from('profiles')
                .upsert({
                    id: data.user.id,
                    username: validUsername,
                    saved_decks: [],
                });

            if (profileError) throw profileError;
            setProfile(await fetchProfile(data.user.id));
        } else if (data.user) {
            setProfile(await fetchProfile(data.user.id));
        }
    }, []);

    const signOut = useCallback(async () => {
        if (!supabase) return;

        const { error } = await supabase.auth.signOut();
        if (error) throw error;
        setProfile(null);
    }, []);

    const updateProfile = useCallback(async (updates: ProfileUpdates) => {
        if (!supabase) throw new Error('Supabase が設定されていません。');
        if (!session?.user) throw new Error('ログインが必要です。');

        const sanitizedUpdates = { ...updates };
        if (typeof sanitizedUpdates.username === 'string') {
            sanitizedUpdates.username = assertValidUsername(sanitizedUpdates.username);
        }

        const { data, error } = await supabase
            .from('profiles')
            .update(sanitizedUpdates)
            .eq('id', session.user.id)
            .select('*')
            .single();

        if (error) throw error;
        setProfile(normalizeProfile(data as Profile));
    }, [session]);

    const refreshProfile = useCallback(async () => {
        if (!session?.user) {
            setProfile(null);
            return;
        }

        setProfile(await fetchProfile(session.user.id));
    }, [session]);

    const value = useMemo<AuthContextValue>(() => ({
        session,
        user: session?.user ?? null,
        profile,
        loading,
        signIn,
        signUp,
        signOut,
        updateProfile,
        refreshProfile,
    }), [loading, profile, refreshProfile, session, signIn, signOut, signUp, updateProfile]);

    return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
    const value = useContext(AuthContext);
    if (!value) {
        throw new Error('useAuth は AuthProvider の中で使ってください。');
    }
    return value;
}
