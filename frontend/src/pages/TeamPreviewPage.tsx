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

type AiTeamEntry = {
    species_id: string;
    moves: string[];
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
    
            const usedIds = new Set(loadedPlayerDeck.map((pokemon) => pokemon.speciesId));
            const speciesList = Object.values(loadedSpecies).filter((mon) => !usedIds.has(mon.id));
            const buildFallbackAiDeck = () =>
                speciesList
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

            let aiDeck: DeckPokemon[] = buildFallbackAiDeck();
            if (currentBattleMode === 'ai' && storedAiLevel === 'lv2') {
                const aiTeamJson = await loadAiTeam();
                const normalizedAiTeam = aiTeamJson ? normalizeAiTeam(aiTeamJson, loadedSpecies) : [];
                if (normalizedAiTeam.length === 3) {
                    aiDeck = normalizedAiTeam;
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
    
        const selectedOpponentDeck =
            battleMode === 'ai' && aiLevel === 'lv2'
                ? opponentDeck.slice(0, SELECT_TEAM_SIZE)
                : pickRandomTeam(opponentDeck, SELECT_TEAM_SIZE);
    
        sessionStorage.setItem('selectedPlayerDeck', JSON.stringify(selectedTeam));
        sessionStorage.setItem('selectedOpponentDeck', JSON.stringify(selectedOpponentDeck));
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
                <div className="mb-6 rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-5">
                    <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
                        <div>
                            <h2 className="text-base font-bold text-[var(--text-primary)]">あなたの選出</h2>
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
                                        onClick={() => setAiLevel('lv1')}
                                        className={`rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                                            aiLevel === 'lv1'
                                                ? 'bg-[var(--accent)] text-white'
                                                : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]'
                                        }`}
                                    >
                                        LV1: Minimax
                                    </button>
                                    <button
                                        type="button"
                                        onClick={() => setAiLevel('lv2')}
                                        className={`rounded-lg px-3 py-2 text-sm font-medium transition-all ${
                                            aiLevel === 'lv2'
                                                ? 'bg-[var(--accent)] text-white'
                                                : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]'
                                        }`}
                                    >
                                        LV2: MLP (進化戦略)
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

                <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-5">
                    <h2 className="mb-1 text-base font-bold text-[var(--text-primary)]">
                        {battleMode === 'ai' && aiLevel === 'lv2' ? '相手の固定チーム' : '相手の6匹'}
                    </h2>
                    <p className="mb-4 text-sm text-[var(--text-muted)]">
                        {battleMode === 'player'
                            ? onlineSnapshot.remoteSelectedDeck
                                ? '相手の選出が完了しました'
                                : '相手もこの中から3匹を選出します'
                            : aiLevel === 'lv2'
                                ? '学習済みのAIチームを使います'
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
                <div className="absolute right-3 top-3 flex size-7 items-center justify-center rounded-full bg-[var(--accent)] text-sm font-bold text-white">
                    {selectedOrder}
                </div>
            ) : onClick ? (
                <div className="absolute right-3 top-3 flex size-7 items-center justify-center rounded-full border border-[var(--border)] text-[var(--text-muted)]">
                    <Check className="size-4" />
                </div>
            ) : null}

            <div className="pr-8">
                <div className="text-lg font-bold text-[var(--text-primary)]">{mon?.name ?? pokemon.speciesId}</div>

                <div className="mt-2 flex flex-wrap gap-1">
                    {(mon?.type ?? []).map((type) => (
                        <span
                            key={type}
                            className="rounded-full px-2 py-0.5 text-xs text-white"
                            style={{ backgroundColor: getTypeColor(type) }}
                        >
                            {getTypeLabel(type)}
                        </span>
                    ))}
                </div>

                {hideMoves ? (
    <div className="mt-3 rounded-lg bg-[var(--surface-2)] px-3 py-2 text-xs text-[var(--text-muted)]">
        技構成は非公開
    </div>
) : (
    <div className="mt-3 grid grid-cols-2 gap-1 text-xs text-[var(--text-muted)]">
        {pokemon.moves.slice(0, 4).map((moveId) => (
            <div key={moveId} className="truncate rounded-lg bg-[var(--surface-2)] px-2 py-1">
                {moves[moveId]?.name ?? moveId}
            </div>
        ))}
    </div>
)}
            </div>
        </button>
    );
}
