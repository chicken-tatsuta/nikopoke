import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft, Check } from 'lucide-react';
import { loadAllData, getTypeColor } from '../lib/data';
import {
    getOnlineSessionSnapshot,
    sendTeamSelected,
    subscribeOnlineSession,
} from '../lib/p2p';
import type { DeckPokemon, MoveData, SpeciesData } from '../types/pokemon';

const SELECT_TEAM_SIZE = 3;

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

function getTypeLabel(type: string): string {
    return TYPE_LABELS[type] ?? type;
}

function pickRandomTeam(deck: DeckPokemon[], count: number): DeckPokemon[] {
    return [...deck]
        .sort(() => Math.random() - 0.5)
        .slice(0, count);
}

function shuffle<T>(items: T[]): T[] {
    return [...items].sort(() => Math.random() - 0.5);
}

type AiTeamEntry = {
    species_id: string;
    moves: string[];
};

type AiTeamCandidate = {
    id?: string;
    generation?: number;
    wins?: number;
    total?: number;
    winRate?: number;
    team: AiTeamEntry[];
};

async function loadAiTeam(): Promise<AiTeamEntry[] | null> {
    try {
        const response = await fetch('/ai_team.json');
        if (!response.ok) {
            return null;
        }
        return (await response.json()) as AiTeamEntry[];
    } catch (error) {
        console.warn('[team-preview] Failed to load ai_team.json:', error);
        return null;
    }
}

async function loadAiTeamPool(): Promise<AiTeamCandidate[]> {
    try {
        const response = await fetch('/ai_teams.json');
        if (!response.ok) {
            return [];
        }

        const candidates = (await response.json()) as AiTeamCandidate[];
        return candidates.filter((candidate) => candidate.team.length === SELECT_TEAM_SIZE);
    } catch (error) {
        console.warn('[team-preview] Failed to load ai_teams.json:', error);
        return [];
    }
}

function pickAiTeamCandidate(candidates: AiTeamCandidate[]): AiTeamCandidate | null {
    if (candidates.length === 0) return null;

    const topCandidates = [...candidates]
        .sort((a, b) => (b.winRate ?? 0) - (a.winRate ?? 0))
        .slice(0, 12);
    return topCandidates[Math.floor(Math.random() * topCandidates.length)] ?? null;
}

function normalizeAiTeam(
    team: AiTeamEntry[],
    loadedSpecies: SpeciesData,
): DeckPokemon[] {
    return team
        .map((entry) => {
            const mon = loadedSpecies[entry.species_id];
            if (!mon) return null;

            return {
                speciesId: mon.id,
                moves: entry.moves.slice(0, 4),
                ability: mon.abilities[0] || 'none',
            };
        })
        .filter((pokemon): pokemon is DeckPokemon => pokemon !== null);
}

function buildFallbackAiDeck(
    loadedSpecies: SpeciesData,
    loadedMoves: MoveData,
    loadedPlayerDeck: DeckPokemon[],
): DeckPokemon[] {
    const usedIds = new Set(loadedPlayerDeck.map((pokemon) => pokemon.speciesId));
    const speciesList = Object.values(loadedSpecies).filter((mon) => !usedIds.has(mon.id));

    return speciesList
        .sort(() => Math.random() - 0.5)
        .slice(0, 6)
        .map((mon) => {
            const fallbackMoves = Object.values(loadedMoves)
                .filter((move) => mon.type.includes(move.type))
                .slice(0, 4)
                .map((move) => move.id);

            const playerFallbackMoves = loadedPlayerDeck[0]?.moves ?? [];

            return {
                speciesId: mon.id,
                moves: fallbackMoves.length > 0 ? fallbackMoves : playerFallbackMoves.slice(0, 4),
                ability: mon.abilities[0] || 'none',
            };
        });
}

async function buildLv2AiDeck(loadedSpecies: SpeciesData): Promise<DeckPokemon[] | null> {
    const candidates = await loadAiTeamPool();
    const primaryCandidate = pickAiTeamCandidate(candidates);
    const candidateTeams = [
        ...(primaryCandidate ? [primaryCandidate] : []),
        ...shuffle(candidates.filter((candidate) => candidate !== primaryCandidate)),
    ];

    const deck: DeckPokemon[] = [];
    const usedSpeciesIds = new Set<string>();

    for (const candidate of candidateTeams) {
        for (const pokemon of normalizeAiTeam(candidate.team, loadedSpecies)) {
            if (usedSpeciesIds.has(pokemon.speciesId)) {
                continue;
            }

            deck.push(pokemon);
            usedSpeciesIds.add(pokemon.speciesId);

            if (deck.length >= 6) {
                return deck;
            }
        }
    }

    if (deck.length >= SELECT_TEAM_SIZE) {
        return deck;
    }

    const fallbackTeam = await loadAiTeam();
    const normalizedFallback = fallbackTeam ? normalizeAiTeam(fallbackTeam, loadedSpecies) : [];
    return normalizedFallback.length >= SELECT_TEAM_SIZE ? normalizedFallback : null;
}

export default function TeamPreviewPage() {
    const navigate = useNavigate();
    const [species, setSpecies] = useState<SpeciesData>({});
    const [moves, setMoves] = useState<MoveData>({});
    const [playerDeck, setPlayerDeck] = useState<DeckPokemon[]>([]);
    const [opponentDeck, setOpponentDeck] = useState<DeckPokemon[]>([]);
    const [selectedIndexes, setSelectedIndexes] = useState<number[]>([]);
    const [battleMode, setBattleMode] = useState<'ai' | 'player'>('ai');
    const [aiLevel, setAiLevel] = useState<'lv1' | 'lv2'>('lv1');
    const [onlineSnapshot, setOnlineSnapshot] = useState(getOnlineSessionSnapshot());
    const [submitted, setSubmitted] = useState(false);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        let cancelled = false;
    
        const boot = async () => {
            const currentBattleMode =
                sessionStorage.getItem('battleMode') === 'player' ? 'player' : 'ai';
            const storedAiLevel = sessionStorage.getItem('aiLevel') === 'lv2' ? 'lv2' : 'lv1';
    
            setBattleMode(currentBattleMode);
            setAiLevel(storedAiLevel);
    
            const { species: loadedSpecies, moves: loadedMoves } = await loadAllData();
    
            if (cancelled) return;
    
            if (currentBattleMode === 'player') {
                const snapshot = getOnlineSessionSnapshot();
    
                if (!snapshot.localDeck || !snapshot.remoteDeck) {
                    navigate('/online-lobby');
                    return;
                }
    
                setSpecies(loadedSpecies);
                setMoves(loadedMoves);
                setPlayerDeck(snapshot.localDeck);
                setOpponentDeck(snapshot.remoteDeck);
                setOnlineSnapshot(snapshot);
                setLoading(false);
                return;
            }
    
            const deckJson = sessionStorage.getItem('playerDeck');
            if (!deckJson) {
                navigate('/deck-builder');
                return;
            }
    
            const loadedPlayerDeck: DeckPokemon[] = JSON.parse(deckJson);
    
            let aiDeck: DeckPokemon[] = buildFallbackAiDeck(loadedSpecies, loadedMoves, loadedPlayerDeck);
            if (currentBattleMode === 'ai' && storedAiLevel === 'lv2') {
                const lv2Deck = await buildLv2AiDeck(loadedSpecies);
                if (lv2Deck) {
                    aiDeck = lv2Deck;
                }
            }
    
            setSpecies(loadedSpecies);
            setMoves(loadedMoves);
            setPlayerDeck(loadedPlayerDeck);
            setOpponentDeck(aiDeck);
            setLoading(false);
        };
    
        boot().catch((error) => {
            console.error('Failed to load team preview:', error);
            navigate('/deck-builder');
        });
    
        return () => {
            cancelled = true;
        };
    }, [navigate]);

    useEffect(() => {
        if (battleMode !== 'player') {
            return;
        }
    
        return subscribeOnlineSession((event) => {
            if (event.type === 'snapshot') {
                setOnlineSnapshot(event.snapshot);
    
                if (event.snapshot.localSelectedDeck && event.snapshot.remoteSelectedDeck) {
                    sessionStorage.setItem(
                        'selectedPlayerDeck',
                        JSON.stringify(event.snapshot.localSelectedDeck),
                    );
                    sessionStorage.setItem(
                        'selectedOpponentDeck',
                        JSON.stringify(event.snapshot.remoteSelectedDeck),
                    );
                    navigate('/battle');
                }
    
                return;
            }
    
            if (event.type === 'team_selected') {
                const snapshot = getOnlineSessionSnapshot();
                setOnlineSnapshot(snapshot);
    
                if (snapshot.localSelectedDeck && snapshot.remoteSelectedDeck) {
                    sessionStorage.setItem(
                        'selectedPlayerDeck',
                        JSON.stringify(snapshot.localSelectedDeck),
                    );
                    sessionStorage.setItem(
                        'selectedOpponentDeck',
                        JSON.stringify(snapshot.remoteSelectedDeck),
                    );
                    navigate('/battle');
                }
            }
        });
    }, [battleMode, navigate]);

    const selectedTeam = useMemo(
        () => selectedIndexes.map((index) => playerDeck[index]).filter(Boolean),
        [playerDeck, selectedIndexes],
    );

    const changeAiLevel = async (nextAiLevel: 'lv1' | 'lv2') => {
        setAiLevel(nextAiLevel);
        sessionStorage.setItem('aiLevel', nextAiLevel);

        if (battleMode !== 'ai' || playerDeck.length === 0 || Object.keys(species).length === 0) {
            return;
        }

        if (nextAiLevel === 'lv2') {
            const lv2Deck = await buildLv2AiDeck(species);
            if (lv2Deck) {
                setOpponentDeck(lv2Deck);
            }
            return;
        }

        setOpponentDeck(buildFallbackAiDeck(species, moves, playerDeck));
    };

    const toggleSelected = (index: number) => {
        setSelectedIndexes((current) => {
            if (current.includes(index)) {
                return current.filter((item) => item !== index);
            }

            if (current.length >= SELECT_TEAM_SIZE) {
                return current;
            }

            return [...current, index];
        });
    };

    const startBattle = () => {
        if (selectedTeam.length !== SELECT_TEAM_SIZE) return;
    
        if (battleMode === 'player') {
            try {
                sendTeamSelected(selectedTeam);
                setSubmitted(true);
    
                const snapshot = getOnlineSessionSnapshot();
                setOnlineSnapshot(snapshot);
    
                if (snapshot.localSelectedDeck && snapshot.remoteSelectedDeck) {
                    sessionStorage.setItem('selectedPlayerDeck', JSON.stringify(snapshot.localSelectedDeck));
                    sessionStorage.setItem('selectedOpponentDeck', JSON.stringify(snapshot.remoteSelectedDeck));
                    navigate('/battle');
                }
            } catch (error) {
                console.error('Failed to submit selected team:', error);
                window.alert(error instanceof Error ? error.message : '選出の送信に失敗しました。');
            }
            return;
        }
    
        const selectedOpponentDeck = aiLevel === 'lv2'
            ? opponentDeck.slice(0, SELECT_TEAM_SIZE)
            : pickRandomTeam(opponentDeck, SELECT_TEAM_SIZE);
    
        sessionStorage.setItem('selectedPlayerDeck', JSON.stringify(selectedTeam));
        sessionStorage.setItem('selectedOpponentDeck', JSON.stringify(selectedOpponentDeck));
        sessionStorage.setItem('opponentPreviewDeck', JSON.stringify(opponentDeck));
        sessionStorage.setItem('aiLevel', aiLevel);
        navigate('/battle');
    };

    if (loading) {
        return (
            <div className="flex min-h-dvh items-center justify-center bg-[var(--surface-1)] text-[var(--text-muted)]">
                読み込み中...
            </div>
        );
    }

    return (
        <div className="min-h-dvh bg-[var(--surface-1)]">
            <header className="sticky top-0 z-20 border-b border-[var(--border)] bg-[var(--surface-2)]">
                <div className="mx-auto flex max-w-6xl items-center gap-4 px-6 py-4">
                    <button
                        onClick={() => navigate('/deck-builder')}
                        className="rounded-lg p-2 transition-colors hover:bg-[var(--surface-3)]"
                        aria-label="デッキ作成に戻る"
                    >
                        <ArrowLeft className="size-5 text-[var(--text-muted)]" />
                    </button>
                    <div>
                        <h1 className="text-xl font-bold text-[var(--text-primary)]">選出</h1>
                        <p className="text-sm text-[var(--text-muted)]">
                            6匹の中から3匹を選んでください
                        </p>
                    </div>
                </div>
            </header>

            <main className="mx-auto max-w-6xl px-6 py-8">
                <div className="mb-6 grid grid-cols-1 gap-6 lg:grid-cols-[1fr_320px]">
                    <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-5">
                        <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
                            <div>
                                <h2 className="text-base font-bold text-[var(--text-primary)]">あなたの6匹</h2>
                                <p className="text-sm text-[var(--text-muted)]">
                                    {selectedIndexes.length}/{SELECT_TEAM_SIZE} 匹選択中
                                </p>
                            </div>

                            {battleMode === 'ai' && (
                                <div className="flex items-center gap-2">
                                    <span className="text-sm font-medium text-[var(--text-primary)]">AIの強さ:</span>
                                    <div className="grid grid-cols-2 gap-2">
                                            <button
                                                type="button"
                                                onClick={() => void changeAiLevel('lv1')}
                                                className={`rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                                                    aiLevel === 'lv1'
                                                        ? 'bg-[var(--accent)] text-white'
                                                    : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]'
                                            }`}
                                        >
                                            LV1: Minimax 1
                                        </button>
                                            <button
                                                type="button"
                                                onClick={() => void changeAiLevel('lv2')}
                                                className={`rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                                                    aiLevel === 'lv2'
                                                        ? 'bg-[var(--accent)] text-white'
                                                    : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]'
                                            }`}
                                        >
                                            LV2: Vega 2
                                        </button>
                                    </div>
                                </div>
                            )}

                            <button
                                    onClick={startBattle}
                                    disabled={selectedIndexes.length !== SELECT_TEAM_SIZE || submitted}
                                className={`rounded-xl px-5 py-3 font-semibold transition-all ${
                                    selectedIndexes.length === SELECT_TEAM_SIZE && !submitted
                                        ? 'bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)]'
                                        : 'cursor-not-allowed bg-[var(--surface-3)] text-[var(--text-muted)]'
                                }`}
                            >
                                {submitted ? '相手の選出を待っています...' : battleMode === 'player' ? 'この3匹を送信' : 'この3匹で対戦'}
                            </button>
                        </div>

                        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                            {playerDeck.map((pokemon, index) => (
                                <TeamCard
                                    key={`${pokemon.speciesId}-${index}`}
                                    pokemon={pokemon}
                                    species={species}
                                    moves={moves}
                                    selectedOrder={selectedIndexes.indexOf(index) + 1}
                                    onClick={() => toggleSelected(index)}
                                />
                            ))}
                        </div>
                    </div>

                    <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-4">
                        <h2 className="mb-3 text-sm font-bold text-[var(--text-primary)]">選択中</h2>
                        <div className="grid grid-cols-1 gap-2">
                            {Array.from({ length: SELECT_TEAM_SIZE }).map((_, slot) => {
                                const deckIndex = selectedIndexes[slot];
                                const pokemon = deckIndex !== undefined ? playerDeck[deckIndex] : null;
                                const mon = pokemon ? species[pokemon.speciesId] : null;

                                return (
                                    <div
                                        key={slot}
                                        className={`rounded-xl border p-3 transition-all ${
                                            pokemon
                                                ? 'border-[var(--accent)] bg-[var(--accent-muted)]'
                                                : 'border-dashed border-[var(--border)] bg-[var(--surface-3)]'
                                        }`}
                                    >
                                        {pokemon && mon ? (
                                            <div className="flex min-h-14 items-center gap-3">
                                                <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-[var(--accent)] text-xs font-bold text-white">
                                                    {slot + 1}
                                                </div>
                                                <div className="min-w-0 text-left">
                                                <div className="truncate text-sm font-bold text-[var(--text-primary)]">
                                                    {mon.name}
                                                </div>
                                                <div className="mt-1 flex flex-wrap gap-1">
                                                    {mon.type.map((type) => (
                                                        <span
                                                            key={type}
                                                            className="rounded-full px-1.5 py-0.5 text-[10px] text-white"
                                                            style={{ backgroundColor: getTypeColor(type) }}
                                                        >
                                                            {getTypeLabel(type)}
                                                        </span>
                                                    ))}
                                                </div>
                                                </div>
                                            </div>
                                        ) : (
                                            <div className="flex min-h-14 items-center gap-3 text-xs text-[var(--text-muted)]">
                                                <div className="flex size-7 shrink-0 items-center justify-center rounded-full border border-dashed border-[var(--border)]">
                                                    {slot + 1}
                                                </div>
                                                未選択
                                            </div>
                                        )}
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                </div>

                <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-5">
                    <h2 className="mb-1 text-base font-bold text-[var(--text-primary)]">
                        相手の6匹
                    </h2>
                    <p className="mb-4 text-sm text-[var(--text-muted)]">
                        {battleMode === 'player'
                            ? onlineSnapshot.remoteSelectedDeck
                                ? '相手の選出が完了しました'
                                : '相手もこの中から3匹を選出します'
                            : 'AIはこの中から3匹を選出します'}
                    </p>

                    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
                        {opponentDeck.map((pokemon, index) => (
                            <TeamCard
                            key={`${pokemon.speciesId}-${index}`}
                            pokemon={pokemon}
                            species={species}
                            moves={moves}
                            hideMoves
                        />
                        ))}
                    </div>
                </div>
            </main>
        </div>
    );
}

function TeamCard({
    pokemon,
    species,
    moves,
    selectedOrder,
    onClick,
    hideMoves = false,
}: {
    pokemon: DeckPokemon;
    species: SpeciesData;
    moves: MoveData;
    selectedOrder?: number;
    onClick?: () => void;
    hideMoves?: boolean;
}) {
    const mon = species[pokemon.speciesId];

    return (
        <button
            type="button"
            onClick={onClick}
            disabled={!onClick}
            className={`relative rounded-2xl border p-4 text-left transition-all ${
                selectedOrder
                    ? 'border-[var(--accent)] bg-[var(--accent-muted)]'
                    : onClick
                        ? 'border-[var(--border)] bg-[var(--surface-3)] hover:border-[var(--border-hover)] hover:bg-[var(--surface-4)]'
                        : 'cursor-default border-[var(--border)] bg-[var(--surface-3)]'
            }`}
        >
            {selectedOrder ? (
                <div className="absolute right-3 top-3 flex size-6 items-center justify-center rounded-full bg-[var(--accent)] text-xs font-bold text-white">
                    {selectedOrder}
                </div>
            ) : onClick ? (
                <div className="absolute right-3 top-3 flex size-6 items-center justify-center rounded-full border border-[var(--border)] text-[var(--text-muted)]">
                    <Check className="size-3.5" />
                </div>
            ) : null}

            <div className="pr-7">
                <div className="text-sm font-bold text-[var(--text-primary)]">{mon?.name ?? pokemon.speciesId}</div>

                <div className="mt-1.5 flex flex-wrap gap-1">
                    {(mon?.type ?? []).map((type) => (
                        <span
                            key={type}
                            className="rounded-full px-1.5 py-0.5 text-[10px] text-white"
                            style={{ backgroundColor: getTypeColor(type) }}
                        >
                            {getTypeLabel(type)}
                        </span>
                    ))}
                </div>

                {hideMoves ? (
                    <div className="mt-2 rounded-lg bg-[var(--surface-2)] px-2 py-1.5 text-[10px] text-[var(--text-muted)]">
                        技構成は非公開
                    </div>
                ) : (
                    <div className="mt-2 grid grid-cols-2 gap-1 text-[10px] text-[var(--text-muted)]">
                        {pokemon.moves.slice(0, 4).map((moveId) => (
                            <div key={moveId} className="truncate rounded bg-[var(--surface-2)] px-1.5 py-0.5">
                                {moves[moveId]?.name ?? moveId}
                            </div>
                        ))}
                    </div>
                )}
            </div>
        </button>
    );
}
