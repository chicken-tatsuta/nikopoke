import { useState, useEffect, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { UserRound } from 'lucide-react';
import { loadSpecies } from '../lib/data';
import type { SpeciesData, Species } from '../types/pokemon';
import type { PokemonUsageStats } from '../lib/battleStats';
import { loadGlobalPokemonUsageStats } from '../lib/globalBattleStats';
import { useAuth } from '../contexts/AuthContext';
import { getPokemonPortraitSrc } from '../lib/pokemonImages';
import heroBackgroundUrl from '../../image/bg.jpg';

const STAT_ROWS = [
    ['hp', 'H'],
    ['atk', 'A'],
    ['def', 'B'],
    ['spa', 'C'],
    ['spd', 'D'],
    ['spe', 'S'],
] as const;

const TYPE_LABELS: Record<string, string> = {
    normal: 'ノーマル',
    fire: 'ほのお',
    water: 'みず',
    electric: 'でんき',
    grass: 'くさ',
    ice: 'こおり',
    fighting: 'かくとう',
    poison: 'どく',
    ground: 'じめん',
    flying: 'ひこう',
    psychic: 'エスパー',
    bug: 'むし',
    rock: 'いわ',
    ghost: 'ゴースト',
    dragon: 'ドラゴン',
    dark: 'あく',
    steel: 'はがね',
    fairy: 'フェアリー',
};

export default function HomePage() {
    const { profile } = useAuth();
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

    const topSpecies = useMemo(() => {
        return Object.values(species)
            .sort((a, b) => {
                const aUsed = usageStats[a.id]?.used ?? 0;
                const bUsed = usageStats[b.id]?.used ?? 0;

                if (aUsed !== bUsed) return bUsed - aUsed;
                return a.name.localeCompare(b.name, 'ja');
            })
            .slice(0, 4);
    }, [species, usageStats]);

    return (
        <div className="min-h-dvh bg-white text-[#111111]">
            <HomeHeader profile={profile} />

            <div className="mx-auto min-h-dvh max-w-7xl overflow-hidden bg-white">
                <main className="mx-auto flex min-h-[calc(100dvh-52px)] max-w-6xl flex-col px-5 py-3 sm:px-8 sm:py-4 lg:px-10">
                    <section className="relative isolate -mx-5 grid items-center gap-5 overflow-hidden px-5 py-8 sm:-mx-8 sm:px-8 sm:py-10 lg:-mx-10 lg:min-h-[450px] lg:px-10">
                        <div className="absolute inset-0 -z-10 bg-black/50" />
                        <div className="absolute inset-0 -z-20 bg-cover bg-center" style={{ backgroundImage: `url(${heroBackgroundUrl})` }} />

                        <div className="max-w-2xl space-y-3 lg:pl-24">
                            <div className="space-y-2.5">
                                <h1 className="text-balance text-3xl font-semibold leading-[1.4] tracking-[0.12em] text-white sm:text-4xl lg:text-[2.5rem]">
                                    二期男が<br />
                                    ニキダンに。
                                </h1>
                                <p className="max-w-md text-xs font-semibold leading-6 tracking-[0.08em] text-white/90">
                                    とある 学生たちを モチーフにした ニキダンたちで 戦え。<br />
                                    もうなんか どっかで みたことある気がする 対戦ゲーム。
                                </p>
                            </div>

                            <div className="flex flex-wrap gap-3">
                                <PrimaryLink to="/pokedex">図鑑を見る</PrimaryLink>
                                <PrimaryLink to="/battle-mode">バトルをはじめる</PrimaryLink>
                            </div>
                        </div>
                    </section>

                    <section id="featured" className="-mx-5 mt-4 flex flex-1 flex-col sm:-mx-8 lg:-mx-10">
                        <div className="mb-2.5 flex items-center justify-between">
                            <div className="flex items-center gap-3">
                                <span className="h-6 w-3 rounded-l-full border border-r-0 border-[#111111] bg-[#F5EEE4]" />
                                <h2 className="text-base font-bold tracking-[0.12em] sm:text-lg">注目のニキダン</h2>
                            </div>
                            <Link to="/pokedex" className="text-sm font-semibold tracking-[0.08em] hover:underline">
                                すべて見る →
                            </Link>
                        </div>

                        {loading ? (
                            <div className="rounded-lg border border-[#111111] py-12 text-center text-sm text-[#707070]">
                                読み込み中...
                            </div>
                        ) : (
                            <div className="grid flex-1 gap-3 md:grid-cols-2 xl:grid-cols-4">
                                {topSpecies.map((mon, index) => {
                                    const used = usageStats[mon.id]?.used ?? 0;
                                    const usageRate = totalUsed > 0 ? (used / totalUsed) * 100 : 0;

                                    return (
                                        <NikimonCard
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
        </div>
    );
}

function HomeHeader({ profile }: { profile: { username: string; rating: number } | null }) {
    return (
        <header className="border-b border-[#111111] bg-white">
            <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-2.5 sm:px-8 lg:px-10">
                <Link to="/home" className="flex items-center gap-4">
                    <span className="h-6 w-12 rounded-t-full border border-b-0 border-[#111111]" />
                    <span className="text-xl font-bold tracking-[0.2em]">Nikidan</span>
                </Link>

                <nav className="hidden items-center gap-8 text-xs font-bold tracking-[0.16em] md:flex">
                    <Link className="border-b border-[#111111] pb-1" to="/home">トップ</Link>
                    <Link to="/pokedex">図鑑</Link>
                    <Link to="/battle-mode">バトル</Link>
                    <Link to="/ranking">ランキング</Link>
                    <span className="flex items-center gap-2">
                        <span className="grid size-7 place-items-center rounded-full border border-[#111111]">
                            <UserRound className="size-4" strokeWidth={1.8} />
                        </span>
                        {profile ? (
                            <span className="tabular-nums">{profile.username} R{profile.rating}</span>
                        ) : (
                            <span>ログイン</span>
                        )}
                    </span>
                </nav>
            </div>
        </header>
    );
}

function PrimaryLink({ to, children }: { to: string; children: string }) {
    return (
        <Link
            to={to}
            className="inline-flex min-w-40 items-center justify-between gap-6 rounded-md border border-[#111111] bg-white px-5 py-2.5 text-sm font-bold tracking-[0.08em] transition-colors hover:bg-[#F5EEE4]"
        >
            {children}
            <span className="text-xl leading-none">→</span>
        </Link>
    );
}

function NikimonCard({
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
    const portraitSrc = getPokemonPortraitSrc(species.id, species.name);
    const stats = species.baseStats;

    return (
        <Link to={`/pokedex/${species.id}`} className="group relative grid h-full grid-rows-[auto_1fr_auto] overflow-hidden rounded-lg border border-[#111111] bg-white transition-colors hover:bg-[#F5EEE4]">
            <div className="flex items-center justify-between border-b border-[#111111] bg-[#FAFAFA] px-3 py-1.5">
                <span className="text-[9px] font-bold tracking-[0.22em]">NIKIDAN RANK</span>
                <span className="text-xs font-bold tabular-nums">RANK.{String(rank).padStart(2, '0')}</span>
            </div>

            <div className="grid grid-cols-[86px_1fr] gap-3 p-3">
                <div className="space-y-1.5">
                    <div className="aspect-[3/3.45] overflow-hidden rounded border border-[#111111] bg-white">
                        <img
                            src={portraitSrc}
                            alt={species.name}
                            className="size-full object-cover"
                            draggable={false}
                        />
                    </div>
                    <p className="border-t border-[#111111] pt-1 text-center text-[8px] font-bold tracking-[0.18em] text-[#555555]">
                        STUDENT
                    </p>
                </div>

                <div className="min-w-0">
                    <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                            <h3 className="truncate text-base font-bold leading-tight tracking-[0.08em]">{species.name}</h3>
                            <p className="mt-0.5 text-[8px] font-bold tracking-[0.2em] text-[#555555]">BATTLE RECORD</p>
                        </div>
                        <span className="mt-0.5 size-3 shrink-0 rounded-full border border-[#111111] bg-[#F5EEE4]" />
                    </div>

                    <div className="mt-2 flex flex-wrap gap-1">
                        {species.type.slice(0, 2).map((type) => (
                            <span key={type} className="rounded border border-[#111111] bg-white px-2 py-0.5 text-[10px] font-bold leading-none">
                                {TYPE_LABELS[type] ?? type}
                            </span>
                        ))}
                    </div>

                    <div className="mt-2 grid grid-cols-[1fr_auto] gap-x-2 gap-y-1 border-y border-dashed border-[#111111]/40 py-1.5 text-[9px] font-bold text-[#333333]">
                        <span>使用率</span>
                        <span className="text-right tabular-nums">{usageRate.toFixed(1)}%</span>
                        <span>使用数</span>
                        <span className="text-right tabular-nums">{used}回</span>
                        {winRate !== undefined && (
                            <>
                                <span>勝率</span>
                                <span className="text-right tabular-nums">{winRate.toFixed(1)}%</span>
                            </>
                        )}
                    </div>
                </div>
            </div>

            <div className="border-t border-[#111111] px-3 py-2">
                <div className="grid grid-cols-2 gap-x-3 gap-y-1">
                    {STAT_ROWS.map(([key, label]) => (
                        <MiniStat key={key} label={label} value={stats[key]} />
                    ))}
                </div>
            </div>
        </Link>
    );
}

function MiniStat({ label, value }: { label: string; value: number }) {
    const width = Math.max(12, Math.min(100, (value / 150) * 100));

    return (
        <div className="grid grid-cols-[0.8rem_1fr_1.45rem] items-center gap-1 text-[9px]">
            <span className="font-semibold">{label}</span>
            <span className="h-px bg-[#111111]" style={{ width: `${width}%` }} />
            <span className="text-right font-semibold tabular-nums">{value}</span>
        </div>
    );
}
