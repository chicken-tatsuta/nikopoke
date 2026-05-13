import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { loadSpecies } from '../lib/data';
import { loadGlobalPokemonUsageStats } from '../lib/globalBattleStats';
import { getPokemonPortraitSrc } from '../lib/pokemonImages';
import type { PokemonUsageStats } from '../lib/battleStats';
import type { Species, SpeciesData } from '../types/pokemon';

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

export default function PokedexPage() {
    const navigate = useNavigate();
    const [species, setSpecies] = useState<SpeciesData>({});
    const [usageStats, setUsageStats] = useState<Record<string, PokemonUsageStats>>({});
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        loadSpecies().then((data) => {
            setSpecies(data);
            setLoading(false);
        });
    }, []);

    useEffect(() => {
        loadGlobalPokemonUsageStats()
            .then((stats) => setUsageStats(stats))
            .catch((error) => {
                console.error('Failed to load global usage stats:', error);
            });
    }, []);

    const totalUsed = useMemo(() => {
        return Object.values(usageStats).reduce((sum, stats) => sum + stats.used, 0);
    }, [usageStats]);

    const sortedSpecies = useMemo(() => {
        return Object.values(species).sort((a, b) => {
            const aUsed = usageStats[a.id]?.used ?? 0;
            const bUsed = usageStats[b.id]?.used ?? 0;
            if (aUsed !== bUsed) return bUsed - aUsed;
            return a.name.localeCompare(b.name, 'ja');
        });
    }, [species, usageStats]);

    return (
        <div className="min-h-dvh bg-white text-[#111111]">
            <header className="border-b border-[#111111] bg-white">
                <div className="mx-auto flex max-w-6xl items-center justify-between px-5 py-3 sm:px-8 lg:px-10">
                    <button
                        type="button"
                        onClick={() => navigate('/home')}
                        className="inline-flex items-center gap-2 text-sm font-bold tracking-[0.08em]"
                    >
                        <ArrowLeft className="size-4" strokeWidth={1.8} />
                        ホーム
                    </button>
                    <span className="text-xl font-bold tracking-[0.2em]">Nikidan</span>
                </div>
            </header>

            <main className="mx-auto max-w-6xl px-5 py-7 sm:px-8 lg:px-10">
                <div className="mb-5 flex items-end justify-between gap-4">
                    <div>
                        <div className="mb-3 flex items-center gap-3">
                            <span className="h-7 w-3.5 rounded-l-full border border-r-0 border-[#111111] bg-[#F5EEE4]" />
                            <p className="text-xs font-bold tracking-[0.22em]">NIKIDAN INDEX</p>
                        </div>
                        <h1 className="text-3xl font-black tracking-[0.16em]">図鑑</h1>
                    </div>
                    <span className="rounded-md border border-[#111111] bg-[#F5EEE4] px-3 py-1 text-sm font-bold tabular-nums">
                        {sortedSpecies.length} 件
                    </span>
                </div>

                {loading ? (
                    <div className="rounded-lg border border-[#111111] py-16 text-center text-sm text-[#666666]">
                        読み込み中...
                    </div>
                ) : (
                    <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                        {sortedSpecies.map((mon, index) => {
                            const used = usageStats[mon.id]?.used ?? 0;
                            const usageRate = totalUsed > 0 ? (used / totalUsed) * 100 : 0;

                            return (
                                <PokedexCard
                                    key={mon.id}
                                    species={mon}
                                    index={index + 1}
                                    used={used}
                                    usageRate={usageRate}
                                    winRate={usageStats[mon.id]?.winRate}
                                />
                            );
                        })}
                    </div>
                )}
            </main>
        </div>
    );
}

function PokedexCard({
    species,
    index,
    used,
    usageRate,
    winRate,
}: {
    species: Species;
    index: number;
    used: number;
    usageRate: number;
    winRate?: number;
}) {
    const portraitSrc = getPokemonPortraitSrc(species.id, species.name);
    const stats = species.baseStats;

    return (
        <Link
            to={`/pokedex/${species.id}`}
            className="grid grid-cols-[92px_1fr] gap-3 rounded-lg border border-[#111111] bg-white p-3 transition-colors hover:bg-[#F5EEE4]"
        >
            <div>
                <div className="aspect-[3/3.45] overflow-hidden rounded-md border border-[#111111] bg-white">
                    <img src={portraitSrc} alt={species.name} className="size-full object-cover" draggable={false} />
                </div>
                <p className="mt-1.5 text-center text-[10px] font-bold tracking-[0.14em] tabular-nums">
                    No.{String(index).padStart(2, '0')}
                </p>
            </div>

            <div className="min-w-0">
                <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                        <h2 className="truncate text-base font-bold tracking-[0.08em]">{species.name}</h2>
                        <div className="mt-1.5 flex flex-wrap gap-1">
                            {species.type.slice(0, 2).map((type) => (
                                <span key={type} className="rounded border border-[#111111] bg-white px-2 py-0.5 text-[10px] font-bold">
                                    {TYPE_LABELS[type] ?? type}
                                </span>
                            ))}
                        </div>
                    </div>
                    <span className="size-3 shrink-0 rounded-full border border-[#111111] bg-[#F5EEE4]" />
                </div>

                <div className="mt-2 grid grid-cols-2 gap-1 border-y border-dashed border-[#111111]/40 py-1.5 text-[10px] font-bold text-[#333333]">
                    <span>{usageRate.toFixed(1)}%</span>
                    <span className="text-right">{used}回</span>
                    {winRate !== undefined && <span className="col-span-2">勝率 {winRate.toFixed(1)}%</span>}
                </div>

                <div className="mt-2 grid grid-cols-2 gap-x-3 gap-y-1">
                    {STAT_ROWS.map(([key, label]) => (
                        <MiniStat key={key} label={label} value={stats[key]} />
                    ))}
                </div>
            </div>
        </Link>
    );
}

function MiniStat({ label, value }: { label: string; value: number }) {
    return (
        <div className="grid grid-cols-[0.8rem_1fr_1.45rem] items-center gap-1 text-[9px]">
            <span className="font-semibold">{label}</span>
            <span className="h-px bg-[#111111]" style={{ width: `${Math.max(24, Math.min(100, value))}%` }} />
            <span className="text-right font-semibold tabular-nums">{value}</span>
        </div>
    );
}
