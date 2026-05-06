import { useState, useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Swords, Users, BookOpen, ChevronRight } from 'lucide-react';
import { loadSpecies, getTypeColor } from '../lib/data';
import type { SpeciesData, Species } from '../types/pokemon';
import type { PokemonUsageStats } from '../lib/battleStats';
import { loadGlobalPokemonUsageStats } from '../lib/globalBattleStats';

const TYPE_LABELS: Record<string, string> = {
    normal: 'ノーマル', fire: 'ほのお', water: 'みず', electric: 'でんき', grass: 'くさ',
    ice: 'こおり', fighting: 'かくとう', poison: 'どく', ground: 'じめん', flying: 'ひこう',
    psychic: 'エスパー', bug: 'むし', rock: 'いわ', ghost: 'ゴースト', dragon: 'ドラゴン',
    dark: 'あく', steel: 'はがね', fairy: 'フェアリー',
};

export default function HomePage() {
    const [species, setSpecies] = useState<SpeciesData>({});
    const [loading, setLoading] = useState(true);
    const [usageStats, setUsageStats] = useState<Record<string, PokemonUsageStats>>({});

    useEffect(() => {
        loadSpecies().then((data) => {
            setSpecies(data);
            setLoading(false);
        });
    }, []);

    useEffect(() => {
        loadGlobalPokemonUsageStats()
            .then((stats) => {
                setUsageStats(stats);
            })
            .catch((error) => {
                console.error('Failed to load global usage stats:', error);
            });
    }, []);

    const totalUsed = useMemo(() => {
        return Object.values(usageStats).reduce((sum, stats) => sum + stats.used, 0);
    }, [usageStats]);

    const speciesList = useMemo(() => {
        return Object.values(species).sort((a, b) => {
            const aUsed = usageStats[a.id]?.used ?? 0;
            const bUsed = usageStats[b.id]?.used ?? 0;

            if (aUsed !== bUsed) {
                return bUsed - aUsed;
            }

            return a.name.localeCompare(b.name, 'ja');
        });
    }, [species, usageStats]);

    return (
        <div className="min-h-dvh bg-[var(--surface-1)]">
            <header className="bg-[var(--surface-2)] border-b border-[var(--border)]">
                <div className="max-w-5xl mx-auto px-6 py-5 flex items-center justify-between">
                    <h1 className="text-xl font-bold text-[var(--text-primary)]">Nikipoke</h1>
                    <span className="text-sm text-[var(--text-muted)]">ようこそ、トレーナー！</span>
                </div>
            </header>

            <main className="max-w-5xl mx-auto px-6 py-10 space-y-12">
                <section>
                    <h2 className="text-lg font-semibold text-[var(--text-primary)] mb-5">バトルモード</h2>
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <Link
                            to="/deck-builder?mode=ai"
                            className="group bg-[var(--surface-2)] border border-[var(--border)] rounded-xl p-5
                                hover:border-[var(--border-hover)] hover:bg-[var(--surface-3)]
                                transition-all duration-150 card-hover"
                        >
                            <div className="flex items-center gap-4">
                                <div className="p-3 bg-[var(--accent-muted)] rounded-lg">
                                    <Swords className="size-6 text-[var(--accent)]" />
                                </div>
                                <div className="flex-1">
                                    <h3 className="text-base font-semibold text-[var(--text-primary)]">VS AI</h3>
                                    <p className="text-sm text-[var(--text-muted)]">AIとバトルする</p>
                                </div>
                                <ChevronRight className="size-5 text-[var(--text-muted)] group-hover:text-[var(--accent)] transition-colors" />
                            </div>
                        </Link>

                        <Link
                            to="/deck-builder?mode=player"
                            className="group bg-[var(--surface-2)] border border-[var(--border)] rounded-xl p-5
                                hover:border-[var(--border-hover)] hover:bg-[var(--surface-3)]
                                transition-all duration-150 card-hover"
                        >
                            <div className="flex items-center gap-4">
                                <div className="p-3 bg-emerald-500/15 rounded-lg">
                                    <Users className="size-6 text-emerald-400" />
                                </div>
                                <div className="flex-1">
                                    <h3 className="text-base font-semibold text-[var(--text-primary)]">VS Player</h3>
                                    <p className="text-sm text-[var(--text-muted)]">PeerJS でルームを作って対戦する</p>
                                </div>
                                <ChevronRight className="size-5 text-[var(--text-muted)] group-hover:text-emerald-400 transition-colors" />
                            </div>
                        </Link>
                    </div>
                </section>

                <section>
                    <div className="flex items-center justify-between mb-5">
                        <h2 className="text-lg font-semibold text-[var(--text-primary)] flex items-center gap-2">
                            <BookOpen className="size-5" />
                            ポケモン図鑑
                        </h2>
                        <span className="text-sm text-[var(--text-muted)] tabular-nums">{speciesList.length}匹</span>
                    </div>

                    {loading ? (
                        <div className="text-center py-16 text-[var(--text-muted)]">読み込み中...</div>
                    ) : (
                        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-4">
                            {speciesList.map((mon, index) => {
                                const used = usageStats[mon.id]?.used ?? 0;
                                const usageRate = totalUsed > 0 ? (used / totalUsed) * 100 : 0;

                                return (
                                    <PokemonCard
                                        key={mon.id}
                                        species={mon}
                                        rank={index + 1}
                                        used={used}
                                        usageRate={usageRate}
                                        winRate={usageStats[mon.id]?.winRate}
                                    />
                                );
                            })}
                        </div>
                    )}
                </section>
            </main>
        </div>
    );
}

function PokemonCard({
    species,
    rank,
    used,
    usageRate,
    winRate,
}: {
    species: Species;
    rank: number;
    used: number;
    usageRate: number;
    winRate?: number;
}) {
    const stats = species.baseStats;
    const maxStat = Math.max(stats.hp, stats.atk, stats.def, stats.spa, stats.spd, stats.spe);

    return (
        <Link
            to={`/pokedex/${species.id}`}
            className="block bg-[var(--surface-2)] border border-[var(--border)] rounded-xl p-5
                hover:border-[var(--border-hover)] hover:bg-[var(--surface-3)] transition-all duration-150 card-hover"
        >
            <div className="flex items-start justify-between gap-3 mb-3">
                <div className="min-w-0">
                    <h3 className="truncate text-base font-semibold text-[var(--text-primary)]">{species.name}</h3>
                    <p className="mt-1 text-xs text-[var(--text-muted)]">
                        使用率 {usageRate.toFixed(1)}%
                        {used > 0 && winRate !== undefined ? ` / 勝率 ${winRate.toFixed(1)}%` : ''}
                    </p>
                    <p className="mt-0.5 text-[10px] text-[var(--text-muted)]">{used}回使用</p>
                </div>

                <span className="shrink-0 rounded-full bg-[var(--accent-muted)] px-2 py-0.5 text-xs font-semibold text-[var(--accent)]">
                    {rank}位
                </span>
            </div>

            <div className="flex gap-1.5 mb-4">
                {species.type.map((t) => (
                    <span
                        key={t}
                        className="px-2.5 py-1 text-xs font-medium text-white rounded-md"
                        style={{ backgroundColor: getTypeColor(t) }}
                    >
                        {TYPE_LABELS[t] ?? t}
                    </span>
                ))}
            </div>

            <div className="space-y-2">
                <StatBar label="H" value={stats.hp} max={255} isMax={stats.hp === maxStat} />
                <StatBar label="A" value={stats.atk} max={255} isMax={stats.atk === maxStat} />
                <StatBar label="B" value={stats.def} max={255} isMax={stats.def === maxStat} />
                <StatBar label="C" value={stats.spa} max={255} isMax={stats.spa === maxStat} />
                <StatBar label="D" value={stats.spd} max={255} isMax={stats.spd === maxStat} />
                <StatBar label="S" value={stats.spe} max={255} isMax={stats.spe === maxStat} />
            </div>
        </Link>
    );
}

function StatBar({ label, value, max, isMax }: { label: string; value: number; max: number; isMax: boolean }) {
    const percentage = (value / max) * 100;

    return (
        <div className="flex items-center gap-2 text-xs">
            <span className={`w-4 tabular-nums ${isMax ? 'text-[var(--accent)] font-semibold' : 'text-[var(--text-muted)]'}`}>
                {label}
            </span>
            <div className="flex-1 h-1.5 bg-[var(--surface-4)] rounded-full overflow-hidden">
                <div
                    className={`h-full rounded-full transition-all ${isMax ? 'bg-[var(--accent)]' : 'bg-[var(--text-muted)]'}`}
                    style={{ width: `${percentage}%` }}
                />
            </div>
            <span className={`w-7 text-right tabular-nums ${isMax ? 'text-[var(--accent)] font-semibold' : 'text-[var(--text-secondary)]'}`}>
                {value}
            </span>
        </div>
    );
}
