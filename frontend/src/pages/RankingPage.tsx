import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, Trophy } from 'lucide-react';
import { supabase } from '../lib/supabase';
import { useAuth } from '../contexts/AuthContext';

type RankingProfile = {
    id: string;
    username: string;
    win_count: number;
    loss_count: number;
};

function getWinRate(profile: RankingProfile): number {
    const total = profile.win_count + profile.loss_count;
    return total > 0 ? (profile.win_count / total) * 100 : 0;
}

export default function RankingPage() {
    const navigate = useNavigate();
    const { user } = useAuth();
    const [profiles, setProfiles] = useState<RankingProfile[]>([]);
    const [loading, setLoading] = useState(() => Boolean(supabase));
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        if (!supabase) {
            return;
        }

        supabase
            .from('profiles')
            .select('id, username, win_count, loss_count')
            .order('win_count', { ascending: false })
            .limit(50)
            .then(({ data, error: loadError }) => {
                if (loadError) {
                    setError('ランキングの読み込みに失敗しました。');
                    console.error('[ranking] Failed to load profiles:', loadError);
                } else {
                    setProfiles((data ?? []) as RankingProfile[]);
                }
                setLoading(false);
            });
    }, []);

    return (
        <div className="min-h-dvh bg-[var(--surface-1)]">
            <header className="border-b border-[var(--border)] bg-[var(--surface-2)]">
                <div className="mx-auto flex max-w-5xl items-center gap-4 px-6 py-4">
                    <button
                        onClick={() => navigate('/home')}
                        className="rounded-lg p-2 transition-colors hover:bg-[var(--surface-3)]"
                        aria-label="ホームに戻る"
                    >
                        <ArrowLeft className="size-5 text-[var(--text-muted)]" />
                    </button>
                    <div>
                        <h1 className="flex items-center gap-2 text-lg font-semibold text-[var(--text-primary)]">
                            <Trophy className="size-5 text-[var(--accent)]" />
                            ランキング
                        </h1>
                        <p className="text-sm text-[var(--text-muted)]">勝利数が多いトレーナー上位50人</p>
                    </div>
                </div>
            </header>

            <main className="mx-auto max-w-5xl px-6 py-8">
                <section className="overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--surface-2)]">
                    <div className="grid grid-cols-[64px_1fr_80px_80px_92px] gap-3 border-b border-[var(--border)] bg-[var(--surface-3)] px-4 py-3 text-xs font-semibold text-[var(--text-muted)]">
                        <span>順位</span>
                        <span>トレーナー</span>
                        <span className="text-right">勝利</span>
                        <span className="text-right">敗北</span>
                        <span className="text-right">勝率</span>
                    </div>

                    {loading ? (
                        <div className="px-4 py-12 text-center text-[var(--text-muted)]">読み込み中...</div>
                    ) : error ? (
                        <div className="px-4 py-12 text-center text-red-300">{error}</div>
                    ) : !supabase ? (
                        <div className="px-4 py-12 text-center text-[var(--text-muted)]">Supabase が設定されていません。</div>
                    ) : profiles.length === 0 ? (
                        <div className="px-4 py-12 text-center text-[var(--text-muted)]">まだランキングデータがありません。</div>
                    ) : (
                        <div className="divide-y divide-[var(--border)]">
                            {profiles.map((profile, index) => {
                                const isCurrentUser = user?.id === profile.id;
                                return (
                                    <div
                                        key={profile.id}
                                        className={`grid grid-cols-[64px_1fr_80px_80px_92px] gap-3 px-4 py-3 text-sm ${isCurrentUser
                                            ? 'bg-[var(--accent-muted)] text-[var(--text-primary)]'
                                            : 'text-[var(--text-secondary)]'
                                            }`}
                                    >
                                        <span className="font-semibold tabular-nums">{index + 1}位</span>
                                        <span className="min-w-0 truncate font-medium text-[var(--text-primary)]">
                                            {profile.username}
                                        </span>
                                        <span className="text-right tabular-nums">{profile.win_count}</span>
                                        <span className="text-right tabular-nums">{profile.loss_count}</span>
                                        <span className="text-right tabular-nums">{getWinRate(profile).toFixed(1)}%</span>
                                    </div>
                                );
                            })}
                        </div>
                    )}
                </section>
            </main>
        </div>
    );
}
