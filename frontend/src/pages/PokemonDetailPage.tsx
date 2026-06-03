import { useEffect, useMemo, useState } from 'react';
import { Link, useParams } from 'react-router-dom';
import { ArrowLeft, BarChart3 } from 'lucide-react';
import { loadAllData } from '../lib/data';
import { getPokemonPortraitSrc } from '../lib/pokemonImages';
import type { MoveData, Species, SpeciesData } from '../types/pokemon';
import type { PokemonUsageStats } from '../lib/battleStats';
import { loadGlobalPokemonUsageStats } from '../lib/globalBattleStats';

const TYPE_LABELS: Record<string, string> = {
    normal: 'ノーマル', fire: 'ほのお', water: 'みず', electric: 'でんき', grass: 'くさ',
    ice: 'こおり', fighting: 'かくとう', poison: 'どく', ground: 'じめん', flying: 'ひこう',
    psychic: 'エスパー', bug: 'むし', rock: 'いわ', ghost: 'ゴースト', dragon: 'ドラゴン',
    dark: 'あく', steel: 'はがね', fairy: 'フェアリー',
};

type UsageItem = {
    name: string;
    rate: number;
};

export default function PokemonDetailPage() {
    const { speciesId } = useParams<{ speciesId: string }>();
    const [speciesData, setSpeciesData] = useState<SpeciesData>({});
    const [movesData, setMovesData] = useState<MoveData>({});
    const [usageStats, setUsageStats] = useState<Record<string, PokemonUsageStats>>({});
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        loadAllData().then(({ species, moves }) => {
            setSpeciesData(species);
            setMovesData(moves);
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

    const sortedSpecies = useMemo(() => {
        return Object.values(speciesData).sort((a, b) => {
            const aUsed = usageStats[a.id]?.used ?? 0;
            const bUsed = usageStats[b.id]?.used ?? 0;
            if (aUsed !== bUsed) return bUsed - aUsed;
            return a.name.localeCompare(b.name, 'ja');
        });
    }, [speciesData, usageStats]);

    const species = speciesId ? speciesData[speciesId] : undefined;
    const rank = species ? sortedSpecies.findIndex((mon) => mon.id === speciesId) + 1 : 0;

    const currentUsage = speciesId ? usageStats[speciesId] : undefined;
    const currentUsageMoves = useMemo(() => {
        if (!currentUsage?.moves.length) {
            return null;
        }
    
        return currentUsage.moves.map((move: { name: string; rate: number }) => ({
            ...move,
            name: movesData[move.name]?.name ?? move.name,
        }));
    }, [currentUsage, movesData]);

    if (loading) {
        return (
            <PageShell>
                <main className="max-w-7xl mx-auto px-6 py-10">
                    <p className="text-[var(--text-muted)]">読み込み中...</p>
                </main>
            </PageShell>
        );
    }

    if (!species) {
        return (
            <PageShell>
                <main className="max-w-7xl mx-auto px-6 py-10 space-y-6">
                    <BackLink />
                    <p className="text-[var(--text-muted)]">ニキダンが見つかりません。</p>
                </main>
            </PageShell>
        );
    }

    return (
        <PageShell>
            <header className="bg-white border-b border-[var(--border)]">
                <div className="max-w-7xl mx-auto px-6 py-5">
                    <BackLink />
                </div>
            </header>

            <main className="max-w-7xl mx-auto px-6 py-8 space-y-8">
                <section className="grid grid-cols-1 lg:grid-cols-[minmax(0,1fr)_420px] gap-6">
                    <PokemonHero species={species} rank={rank} />
                    <BaseStatsPanel species={species} />
                </section>

                <section>
    <div className="mb-4 flex items-center gap-3">
        <span className="h-7 w-3.5 rounded-l-full border border-r-0 border-[#111111] bg-[#F5EEE4]" />
        <h2 className="text-xl font-bold tracking-[0.12em]">
            「{species.name}」の個体レポート
        </h2>
    </div>

    {currentUsage && (
        <div className="mb-5 flex flex-wrap items-center gap-3 text-sm font-semibold text-[var(--text-secondary)]">
            <span className="rounded-md border border-[var(--border)] bg-[#F5EEE4] px-3 py-1">
                使用数 {currentUsage.used}
            </span>
            <span className="rounded-md border border-[var(--border)] bg-white px-3 py-1">
                勝率 {currentUsage.winRate.toFixed(1)}%
            </span>
            <span className="rounded-md border border-[var(--border)] bg-white px-3 py-1">
                {currentUsage.wins}勝 {currentUsage.losses}敗
            </span>
        </div>
    )}

    <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-6 items-stretch">
    <UsageCard
    title="わざ"
    icon="MV"
    items={currentUsageMoves?.length ? currentUsageMoves : mockMovesFor(species)}
/>
        <UsageCard title="とくせい" icon="AB" items={mockAbilitiesFor(species)} />
        <UsageCard title="せいかく" icon="NT" items={mockNatures} />
        <UsageCard title="もちもの" icon="IT" items={mockItems} />
    </div>
</section>
            </main>
        </PageShell>
    );
}

function PageShell({ children }: { children: React.ReactNode }) {
    return <div className="min-h-dvh bg-white text-[var(--text-primary)]">{children}</div>;
}

function BackLink() {
    return (
        <Link
            to="/home"
            className="inline-flex items-center gap-2 text-sm font-semibold text-[var(--text-muted)] hover:text-[var(--accent)] transition-colors"
        >
            <ArrowLeft className="size-4" />
            図鑑へ戻る
        </Link>
    );
}

function PokemonHero({ species, rank }: { species: Species; rank: number }) {
    const portraitSrc = getPokemonPortraitSrc(species.id, species.name);

    return (
        <div className="bg-white border border-[var(--border)] rounded-lg p-5">
            <div className="flex items-start gap-5">
                <div className="size-32 shrink-0 overflow-hidden rounded-md border border-[var(--border)] bg-[var(--surface-3)]">
                    <img
                        src={portraitSrc}
                        alt={species.name}
                        className="size-full object-cover"
                    />
                </div>

                <div className="flex-1 min-w-0 space-y-4">
                    <div>
                        <div className="flex flex-wrap items-center gap-3">
                            <h1 className="text-3xl font-bold tracking-[0.1em]">{species.name}</h1>
                            {rank > 0 && (
                                <span className="inline-flex items-center gap-1 rounded-md border border-[var(--border)] bg-[var(--accent-muted)] px-2.5 py-1 text-sm font-semibold text-[var(--accent)]">
                                    <BarChart3 className="size-4" />
                                    {rank}位
                                </span>
                            )}
                        </div>

                        <div className="flex flex-wrap gap-2 mt-3">
                            {species.type.map((type) => (
                                <span
                                    key={type}
                                    className="px-3 py-1 text-sm font-bold text-[#111111] rounded-md border border-[#111111] bg-white"
                                >
                                    {TYPE_LABELS[type] ?? type}
                                </span>
                            ))}
                        </div>
                    </div>

                    <div className="whitespace-pre-line rounded-md bg-[var(--surface-3)] border border-dashed border-[var(--border)] p-4 text-sm text-[var(--text-secondary)] leading-relaxed">
                        {species.description || '説明文はまだありません。'}
                    </div>
                </div>
            </div>
        </div>
    );
}

function BaseStatsPanel({ species }: { species: Species }) {
    const stats = species.baseStats;
    const total = stats.hp + stats.atk + stats.def + stats.spa + stats.spd + stats.spe;

    return (
        <div className="bg-white border border-[var(--border)] rounded-lg p-5">
            <h2 className="text-lg font-bold mb-4 tracking-[0.12em]">種族値</h2>
            <div className="space-y-3">
                <BaseStatBar label="HP" value={stats.hp} max={180} />
                <BaseStatBar label="こうげき" value={stats.atk} max={180} />
                <BaseStatBar label="ぼうぎょ" value={stats.def} max={180} />
                <BaseStatBar label="とくこう" value={stats.spa} max={180} />
                <BaseStatBar label="とくぼう" value={stats.spd} max={180} />
                <BaseStatBar label="すばやさ" value={stats.spe} max={180} />
                <BaseStatBar label="合計" value={total} max={720} />
            </div>
        </div>
    );
}

function BaseStatBar({ label, value, max }: { label: string; value: number; max: number }) {
    const percentage = Math.min(100, (value / max) * 100);

    return (
        <div className="grid grid-cols-[76px_1fr_44px] items-center gap-3 text-sm">
            <span className="text-[var(--text-muted)]">{label}</span>
            <div className="h-1.5 bg-[var(--surface-4)] overflow-hidden">
                <div className="h-full bg-[var(--accent)]" style={{ width: `${percentage}%` }} />
            </div>
            <span className="text-right tabular-nums font-semibold">{value}</span>
        </div>
    );
}

function UsageCard({ title, icon, items }: { title: string; icon: string; items: UsageItem[] }) {
    return (
        <article className="h-full min-h-[320px] bg-white border border-[var(--border)] rounded-lg p-5 flex flex-col">
            <div className="flex items-center justify-between gap-2 border-b border-[var(--border)] pb-3 mb-5">
                <h3 className="text-lg font-bold text-[var(--text-primary)] tracking-[0.12em]">{title}</h3>
                <span className="rounded border border-[var(--border)] bg-[#F5EEE4] px-2 py-1 text-[10px] font-bold leading-none">{icon}</span>
            </div>

            <div className="flex-1 space-y-4">
                {items.map((item) => (
                    <UsageRow key={item.name} item={item} />
                ))}
            </div>

            <button
                type="button"
                className="mt-6 w-full rounded-md border border-[var(--border)] bg-white px-4 py-3 text-sm font-bold text-[var(--text-primary)]
                    hover:bg-[#F5EEE4] hover:border-[var(--border-hover)] transition-colors"
            >
                リスト表示
            </button>
        </article>
    );
}

function UsageRow({ item }: { item: UsageItem }) {
    const percentage = Math.max(0, Math.min(100, item.rate));

    return (
        <div className="space-y-2">
            <div className="flex items-center justify-between gap-3 text-sm">
                <span className="min-w-0 truncate text-[var(--text-primary)]">{item.name}</span>
                <span className="shrink-0 tabular-nums text-[var(--text-secondary)]">{item.rate.toFixed(1)}%</span>
            </div>

            <div className="h-1.5 bg-[var(--surface-4)] overflow-hidden">
                <div className="h-full bg-[var(--accent)]" style={{ width: `${percentage}%` }} />
            </div>
        </div>
    );
}

function mockMovesFor(species: Species): UsageItem[] {
    return [
        { name: `${species.name}の主力技`, rate: 69.4 },
        { name: 'ビルドアップ', rate: 45.6 },
        { name: 'まもる', rate: 36.2 },
        { name: 'みがわり', rate: 34.8 },
        { name: 'テラバースト', rate: 25.2 },
    ];
}

const ABILITY_LABELS: Record<string, string> = {
    none: 'なし',

    // 40 abilities from species.yaml (corrected keys)
    berserk: 'ぎゃくじょう',
    chlorophyll: 'ようりょくそ',
    competitive: 'かちき',
    compound_eyes: 'ふくがん',
    contrary: 'あまのじゃく',
    cotton_down: 'わたげ',
    download: 'ダウンロード',
    aroma_veil: 'アロマベール',
    drought: 'ひでり',
    drizzle: 'あめふらし',
    fur_coat: 'ファーコート',
    guts: 'こんじょう',
    hustle: 'はりきり',
    immunity: 'めんえき',
    inner_focus: 'せいしんりょく',
    insomnia: 'ふみん',
    intimidate: 'いかく',
    klutz: 'ぶきよう',
    libero: 'リベロ',
    lightning_rod: 'ひらいしん',
    magic_bounce: 'マジックミラー',
    merciless: 'ひとでなし',
    moody: 'ムラっけ',
    opportunist: 'びんじょう',
    own_tempo: 'マイペース',
    parental_bond: 'おやこあい',
    power_of_alchemy: 'かがくのちから',
    prankster: 'いたずらごころ',
    pure_power: 'ヨガパワー',
    receiver: 'レシーバー',
    sand_stream: 'すなおこし',
    shadow_tag: 'かげふみ',
    simple: 'たんじゅん',
    slow_start: 'スロースタート',
    stamina: 'じきゅうりょく',
    static: 'せいでんき',
    popping_habanero: 'とびだすハバネロ',
    steelworker: 'はがねつかい',
    steely_spirit: 'はがねのせいしん',
    super_luck: 'きょううん',
    swift_swim: 'すいすい',
    technician: 'テクニシャン',
    thick_fat: 'あついしぼう',
    unaware: 'てんねん',
    clear_body: 'クリアボディ',
    unburden: 'かるわざ',
    unnerve: 'きんちょうかん',
    disguise: 'ばけのかわ',
    rattled: 'びびり',
    hospitality: 'おもてなし',
    frisk: 'おみとおし',
    early_bird: 'はやおき',
    sniper: 'スナイパー',

    // Other abilities
    torrent: 'げきりゅう',
    blaze: 'もうか',
    overgrow: 'しんりょく',
    swarm: 'むしのしらせ',
    levitate: 'ふゆう',
    pressure: 'プレッシャー',
    sturdy: 'がんじょう',
    huge_power: 'ちからもち',
    moxie: 'じしんかじょう',
    magic_guard: 'マジックガード',
    adaptability: 'てきおうりょく',
};

function mockAbilitiesFor(species: Species): UsageItem[] {
    const abilities = species.abilities.length > 0 ? species.abilities : ['none'];

    return abilities.map((ability, index) => ({
        name: getAbilityLabel(ability),
        rate: index === 0 ? 100 : Math.max(5, 40 - index * 12),
    }));
}

// eslint-disable-next-line react-refresh/only-export-components
export function getAbilityLabel(abilityId: string): string {
    return ABILITY_LABELS[abilityId] ?? abilityId;
}

const mockNatures: UsageItem[] = [
    { name: 'ようき', rate: 50.4 },
    { name: 'いじっぱり', rate: 49.6 },
];

const mockItems: UsageItem[] = [
    { name: 'こだわりスカーフ', rate: 28.5 },
    { name: 'たべのこし', rate: 24.2 },
    { name: 'きあいのタスキ', rate: 19.8 },
    { name: 'オボンのみ', rate: 12.1 },
];
