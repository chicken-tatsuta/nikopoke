import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, Trophy } from 'lucide-react';
import { supabase } from '../lib/supabase';
import { useAuth } from '../contexts/AuthContext';

type RankingProfile = {
    id: string;
    username: string;
    rating: number;
    win_count: number;
    loss_count: number;
};

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
            .select('id, username, rating, win_count, loss_count')
            .order('rating', { ascending: false })
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
                <div className="mx-auto flex max-w-5xl items-center gap-3 px-3 py-4 sm:gap-4 sm:px-6">
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
                        <p className="text-sm text-[var(--text-muted)]">レートが高いトレーナー上位50人</p>
                    </div>
                </div>
            </header>

            <main className="mx-auto max-w-5xl px-3 py-4 sm:px-6 sm:py-8">
                <section className="overflow-hidden rounded-xl border border-[var(--border)] bg-[var(--surface-2)]">
                    <div className="max-h-[calc(100dvh-150px)] overflow-auto">
                        <div className="min-w-[330px]">
                            <div className="sticky top-0 z-10 grid grid-cols-[44px_minmax(0,1fr)_64px_42px_42px] gap-2 border-b border-[var(--border)] bg-[var(--surface-3)] px-3 py-3 text-xs font-semibold text-[var(--text-muted)] sm:grid-cols-[64px_1fr_92px_80px_80px] sm:gap-3 sm:px-4">
                                <span>順位</span>
                                <span>トレーナー</span>
                                <span className="text-right">レート</span>
                                <span className="text-right">勝利</span>
                                <span className="text-right">敗北</span>
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
                                                className={`grid grid-cols-[44px_minmax(0,1fr)_64px_42px_42px] gap-2 px-3 py-3 text-xs sm:grid-cols-[64px_1fr_92px_80px_80px] sm:gap-3 sm:px-4 sm:text-sm ${isCurrentUser
                                                    ? 'bg-[var(--accent-muted)] text-[var(--text-primary)]'
                                                    : 'text-[var(--text-secondary)]'
                                                    }`}
                                            >
                                                <span className="font-semibold tabular-nums">{index + 1}位</span>
                                                <span className="min-w-0 truncate font-medium text-[var(--text-primary)]">
                                                    {profile.username}
                                                </span>
                                                <span className="text-right font-semibold tabular-nums text-[var(--text-primary)]">{profile.rating ?? 1500}</span>
                                                <span className="text-right tabular-nums">{profile.win_count}</span>
                                                <span className="text-right tabular-nums">{profile.loss_count}</span>
                                            </div>
                                        );
                                    })}
                                </div>
                            )}
                        </div>
                    </div>
                </section>
            </main>
        </div>
    );
}
