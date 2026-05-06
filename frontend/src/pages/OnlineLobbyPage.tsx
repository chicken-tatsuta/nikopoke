import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, Copy, RotateCcw, Swords, Users, Wifi } from 'lucide-react';
import { loadSpecies } from '../lib/data';
import {
    clearOnlineSession,
    createHostSession,
    getOnlineSessionSnapshot,
    joinHostSession,
    startOnlineBattle,
    subscribeOnlineSession,
} from '../lib/p2p';
import { useAuth } from '../contexts/AuthContext';
import type { DeckPokemon, SpeciesData } from '../types/pokemon';

function readPlayerDeck(): DeckPokemon[] | null {
    const deckJson = sessionStorage.getItem('playerDeck');
    if (!deckJson) {
        return null;
    }
    try {
        return JSON.parse(deckJson) as DeckPokemon[];
    } catch {
        return null;
    }
}

function describeStatus(role: string | null, status: string, hasRemoteDeck: boolean): string {
    if (status === 'hosting') {
        return 'ルームを作成しました。相手の参加を待っています。';
    }
    if (status === 'joining') {
        return 'ホストへの接続を試しています...';
    }
    if (status === 'connected' || status === 'ready') {
        if (role === 'host' && hasRemoteDeck) {
            return '相手が接続しました。準備ができたら対戦を開始できます。';
        }
        if (role === 'guest' && hasRemoteDeck) {
            return 'ホストと接続しました。対戦開始を待っています。';
        }
        return '接続は完了しました。相手の準備を待っています。';
    }
    if (status === 'in_battle') {
        return '対戦ページへ移動します...';
    }
    if (status === 'disconnected') {
        return '接続が切れました。やり直してください。';
    }
    if (status === 'error') {
        return '接続エラーが発生しました。';
    }
    return 'ルームを作成するか、ルームコードを入力して参加してください。';
}

export default function OnlineLobbyPage() {
    const navigate = useNavigate();
    const { user } = useAuth();
    const [species, setSpecies] = useState<SpeciesData>({});
    const [loadingSpecies, setLoadingSpecies] = useState(true);
    const [session, setSession] = useState(getOnlineSessionSnapshot());
    const [joinCode, setJoinCode] = useState('');
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [copied, setCopied] = useState(false);

    const playerDeck = useMemo(() => readPlayerDeck(), []);

    useEffect(() => {
        if (sessionStorage.getItem('battleMode') !== 'player') {
            sessionStorage.setItem('battleMode', 'player');
        }
        if (!playerDeck) {
            navigate('/deck-builder?mode=player');
        }
    }, [navigate, playerDeck]);

    useEffect(() => {
        loadSpecies()
            .then((data) => {
                setSpecies(data);
                setLoadingSpecies(false);
            })
            .catch(() => {
                setLoadingSpecies(false);
            });
    }, []);

    useEffect(() => {
        return subscribeOnlineSession((event) => {
            if (event.type === 'snapshot') {
                setSession(event.snapshot);
                return;
            }
            if (event.type === 'start_battle') {
                navigate('/team-preview');
                return;
            }
            if (event.type === 'error') {
                setError(event.message);
            }
        });
    }, [navigate]);

    const roomCode = session.hostPeerId ?? session.localPeerId ?? '';
    const localDeck = session.localDeck ?? playerDeck ?? [];
    const remoteDeck = session.remoteDeck ?? [];
    const statusText = describeStatus(session.role, session.status, remoteDeck.length > 0);

    const handleCreateRoom = async () => {
        if (!playerDeck || busy) {
            return;
        }
        setBusy(true);
        setError(null);
        try {
            await createHostSession(playerDeck, user?.id ?? null);
        } catch (createError) {
            const message = createError instanceof Error ? createError.message : 'ルーム作成に失敗しました。';
            setError(message);
        } finally {
            setBusy(false);
        }
    };

    const handleJoinRoom = async () => {
        if (!playerDeck || busy) {
            return;
        }
        if (!joinCode.trim()) {
            setError('ルームコードを入力してください。');
            return;
        }
        setBusy(true);
        setError(null);
        try {
            await joinHostSession(joinCode.trim(), playerDeck, user?.id ?? null);
        } catch (joinError) {
            const message = joinError instanceof Error ? joinError.message : 'ルーム参加に失敗しました。';
            setError(message);
        } finally {
            setBusy(false);
        }
    };

    const handleCopyCode = async () => {
        if (!roomCode) {
            return;
        }
        try {
            await navigator.clipboard.writeText(roomCode);
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1500);
        } catch {
            setError('ルームコードのコピーに失敗しました。');
        }
    };

    const handleStartBattle = () => {
        try {
            startOnlineBattle();
            navigate('/team-preview');
        } catch (startError) {
            const message = startError instanceof Error ? startError.message : '対戦開始に失敗しました。';
            setError(message);
        }
    };

    const handleReset = () => {
        clearOnlineSession();
        setSession(getOnlineSessionSnapshot());
        setJoinCode('');
        setBusy(false);
        setCopied(false);
        setError(null);
    };

    const localNames = localDeck
        .map((pokemon) => species[pokemon.speciesId]?.name ?? pokemon.speciesId)
        .join(' / ');
    const remoteNames = remoteDeck
        .map((pokemon) => species[pokemon.speciesId]?.name ?? pokemon.speciesId)
        .join(' / ');

    return (
        <div className="min-h-dvh bg-[var(--surface-1)]">
            <header className="border-b border-[var(--border)] bg-[var(--surface-2)]">
                <div className="mx-auto flex max-w-5xl items-center justify-between px-6 py-4">
                    <div className="flex items-center gap-4">
                        <button
                            onClick={() => navigate('/deck-builder?mode=player')}
                            className="rounded-lg p-2 transition-colors hover:bg-[var(--surface-3)]"
                            aria-label="デッキ作成へ戻る"
                        >
                            <ArrowLeft className="size-5 text-[var(--text-muted)]" />
                        </button>
                        <div>
                            <h1 className="text-lg font-semibold text-[var(--text-primary)]">オンライン対戦</h1>
                            <p className="text-sm text-[var(--text-muted)]">PeerJS でシンプルに部屋を作って対戦します</p>
                        </div>
                    </div>
                    <button
                        onClick={handleReset}
                        className="inline-flex items-center gap-2 rounded-lg border border-[var(--border)] px-3 py-2 text-sm text-[var(--text-primary)] transition-colors hover:bg-[var(--surface-3)]"
                    >
                        <RotateCcw className="size-4" />
                        やり直す
                    </button>
                </div>
            </header>

            <main className="mx-auto grid max-w-5xl gap-6 px-6 py-8 lg:grid-cols-[1.2fr_0.8fr]">
                <section className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-6">
                    <div className="flex items-center gap-3">
                        <div className="rounded-xl bg-[var(--accent-muted)] p-3">
                            <Wifi className="size-5 text-[var(--accent)]" />
                        </div>
                        <div>
                            <h2 className="text-base font-semibold text-[var(--text-primary)]">接続状態</h2>
                            <p className="text-sm text-[var(--text-muted)]">{statusText}</p>
                        </div>
                    </div>

                    <div className="mt-6 grid gap-4 md:grid-cols-2">
                        <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-3)] p-4">
                            <h3 className="text-sm font-semibold text-[var(--text-primary)]">ルームを作成</h3>
                            <p className="mt-1 text-sm text-[var(--text-muted)]">
                                あなたがホストになり、表示されたコードを相手に共有します。
                            </p>
                            <button
                                onClick={handleCreateRoom}
                                disabled={busy || session.role === 'host'}
                                className="mt-4 inline-flex items-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-3 text-sm font-semibold text-white transition-colors hover:bg-[var(--accent-hover)] disabled:cursor-not-allowed disabled:opacity-50"
                            >
                                <Users className="size-4" />
                                ルームを作る
                            </button>
                        </div>

                        <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-3)] p-4">
                            <h3 className="text-sm font-semibold text-[var(--text-primary)]">ルームに参加</h3>
                            <p className="mt-1 text-sm text-[var(--text-muted)]">
                                相手から受け取ったルームコードを入力して接続します。
                            </p>
                            <div className="mt-4 flex gap-2">
                                <input
                                    value={joinCode}
                                    onChange={(event) => setJoinCode(event.target.value)}
                                    placeholder="ルームコード"
                                    className="flex-1 rounded-xl border border-[var(--border)] bg-[var(--surface-2)] px-3 py-3 text-sm text-[var(--text-primary)] outline-none transition-colors placeholder:text-[var(--text-muted)] focus:border-[var(--accent)]"
                                />
                                <button
                                    onClick={handleJoinRoom}
                                    disabled={busy || session.role === 'guest'}
                                    className="rounded-xl border border-[var(--border)] px-4 py-3 text-sm font-semibold text-[var(--text-primary)] transition-colors hover:bg-[var(--surface-2)] disabled:cursor-not-allowed disabled:opacity-50"
                                >
                                    参加
                                </button>
                            </div>
                        </div>
                    </div>

                    {roomCode && (
                        <div className="mt-6 rounded-xl border border-[var(--accent)]/30 bg-[var(--accent-muted)] p-4">
                            <p className="text-sm text-[var(--text-muted)]">共有するルームコード</p>
                            <div className="mt-2 flex items-center gap-3">
                                <code className="rounded-lg bg-[var(--surface-1)] px-3 py-2 text-sm font-semibold text-[var(--text-primary)]">
                                    {roomCode}
                                </code>
                                <button
                                    onClick={handleCopyCode}
                                    className="inline-flex items-center gap-2 rounded-lg border border-[var(--border)] px-3 py-2 text-sm text-[var(--text-primary)] transition-colors hover:bg-[var(--surface-3)]"
                                >
                                    <Copy className="size-4" />
                                    {copied ? 'コピー済み' : 'コピー'}
                                </button>
                            </div>
                        </div>
                    )}

                    {session.role === 'host' && remoteDeck.length > 0 && (
                        <div className="mt-6">
                            <button
                                onClick={handleStartBattle}
                                className="inline-flex items-center gap-2 rounded-xl bg-emerald-600 px-5 py-3 font-semibold text-white transition-colors hover:bg-emerald-500"
                            >
                                <Swords className="size-4" />
                                対戦を開始する
                            </button>
                        </div>
                    )}

                    {error && (
                        <p className="mt-4 rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-3 text-sm text-red-200">
                            {error}
                        </p>
                    )}
                </section>

                <aside className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-6">
                    <h2 className="text-base font-semibold text-[var(--text-primary)]">参加メンバー</h2>
                    <div className="mt-4 space-y-4">
                        <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-3)] p-4">
                            <p className="text-xs uppercase tracking-wide text-[var(--text-muted)]">あなた</p>
                            <p className="mt-2 text-sm font-medium text-[var(--text-primary)]">
                                {loadingSpecies ? '読み込み中...' : localNames || 'デッキ未設定'}
                            </p>
                        </div>
                        <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-3)] p-4">
                            <p className="text-xs uppercase tracking-wide text-[var(--text-muted)]">相手</p>
                            <p className="mt-2 text-sm font-medium text-[var(--text-primary)]">
                                {remoteDeck.length > 0
                                    ? (loadingSpecies ? '読み込み中...' : remoteNames)
                                    : 'まだ接続されていません'}
                            </p>
                        </div>
                    </div>
                </aside>
            </main>
        </div>
    );
}
