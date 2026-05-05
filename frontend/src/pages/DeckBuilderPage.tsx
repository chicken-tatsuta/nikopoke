import { useState, useEffect } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { Check, X, ArrowLeft, Swords, Sliders } from 'lucide-react';
import { loadAllData, getTypeColor } from '../lib/data';
import { clearOnlineSession } from '../lib/p2p';
import type { SpeciesData, MoveData, Learnset, Species, DeckPokemon, EVStats } from '../types/pokemon';
import { getPokemonPreset, resolvePresetMoveIds } from '../lib/pokemonPresets';
import { getAbilityLabel } from './PokemonDetailPage';

const DECK_SIZE = 6;

export default function DeckBuilderPage() {
    const navigate = useNavigate();
    const [searchParams] = useSearchParams();
    const mode = searchParams.get('mode') || 'ai';

    const [species, setSpecies] = useState<SpeciesData>({});
    const [moves, setMoves] = useState<MoveData>({});
    const [learnsets, setLearnsets] = useState<Learnset>({});
    const [loading, setLoading] = useState(true);

    const [selectedPokemon, setSelectedPokemon] = useState<DeckPokemon[]>([]);
    const [editingIndex, setEditingIndex] = useState<number | null>(null);
    const [editingMoves, setEditingMoves] = useState<string[]>([]);
    const [editingEVIndex, setEditingEVIndex] = useState<number | null>(null);
    const [editingEVs, setEditingEVs] = useState<EVStats>({ hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 });

    useEffect(() => {
        loadAllData().then(({ species, moves, learnsets }) => {
            setSpecies(species);
            setMoves(moves);
            setLearnsets(learnsets);
            setLoading(false);

            // Load saved deck from localStorage after data is loaded
            const saved = localStorage.getItem('savedDeck');
            if (saved) {
                try {
                    const parsed = JSON.parse(saved) as DeckPokemon[];
                    const validPokemon = parsed
                        .map((pokemon) => sanitizeDeckPokemon(pokemon, species, moves, learnsets))
                        .filter((pokemon): pokemon is DeckPokemon => pokemon !== null);
                    if (validPokemon.length > 0) {
                        setSelectedPokemon(validPokemon);
                        localStorage.setItem('savedDeck', JSON.stringify(validPokemon));
                    } else {
                        localStorage.removeItem('savedDeck');
                    }
                } catch (e) {
                    console.error('Failed to load saved deck:', e);
                }
            }
        });
    }, []);

    const speciesList = Object.values(species);

    const handleSelectPokemon = (mon: Species) => {
        if (selectedPokemon.length >= DECK_SIZE) return;
        if (selectedPokemon.some(p => p.speciesId === mon.id)) return;
    
        const preset = getPokemonPreset(mon.id);
        const monLearnset = learnsets[mon.id] || [];
        const presetMoves = resolvePresetMoveIds(preset, moves, monLearnset);
    
        const newPokemon: DeckPokemon = {
            speciesId: mon.id,
            moves: presetMoves,
            ability: mon.abilities[0] || 'none',
            evs: preset?.evs ?? { hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 },
        };
    
        setSelectedPokemon([...selectedPokemon, newPokemon]);
    };

    // Save deck to localStorage whenever it changes
    useEffect(() => {
        if (!loading && selectedPokemon.length > 0) {
            localStorage.setItem('savedDeck', JSON.stringify(selectedPokemon));
        }
    }, [selectedPokemon, loading]);

    const handleRemovePokemon = (index: number) => {
        setSelectedPokemon(selectedPokemon.filter((_, i) => i !== index));
        if (editingIndex === index) {
            setEditingIndex(null);
        }
    };

    const handleEditMoves = (index: number) => {
        setEditingIndex(index);
        setEditingMoves([...selectedPokemon[index].moves]);
    };

    const handleToggleMove = (moveId: string) => {
        if (editingMoves.includes(moveId)) {
            setEditingMoves(editingMoves.filter(m => m !== moveId));
        } else if (editingMoves.length < 4) {
            setEditingMoves([...editingMoves, moveId]);
        }
    };

    const handleSaveMoves = () => {
        if (editingIndex === null) return;
        const updated = [...selectedPokemon];
        updated[editingIndex] = { ...updated[editingIndex], moves: editingMoves };
        setSelectedPokemon(updated);
        setEditingIndex(null);
    };

    const handleEditEVs = (index: number) => {
        setEditingEVIndex(index);
        setEditingEVs(selectedPokemon[index].evs || { hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 });
    };

    const handleSaveEVs = () => {
        if (editingEVIndex === null) return;
        const updated = [...selectedPokemon];
        updated[editingEVIndex] = { ...updated[editingEVIndex], evs: editingEVs };
        setSelectedPokemon(updated);
        setEditingEVIndex(null);
    };

    const handleAbilityChange = (index: number, ability: string) => {
        const updated = [...selectedPokemon];
        updated[index] = { ...updated[index], ability };
        setSelectedPokemon(updated);
    };

    const handleStartBattle = () => {
        if (selectedPokemon.length < DECK_SIZE) return;
        const sanitizedDeck = selectedPokemon
            .map((pokemon) => sanitizeDeckPokemon(pokemon, species, moves, learnsets))
            .filter((pokemon): pokemon is DeckPokemon => pokemon !== null);

        if (sanitizedDeck.length < DECK_SIZE || sanitizedDeck.some((pokemon) => pokemon.moves.length === 0)) {
            window.alert('デッキに古い技データが残っています。技を確認して保存し直してください。');
            return;
        }

        setSelectedPokemon(sanitizedDeck);
        if (mode === 'player') {
            clearOnlineSession();
        }
        localStorage.setItem('savedDeck', JSON.stringify(sanitizedDeck));
        sessionStorage.setItem('playerDeck', JSON.stringify(sanitizedDeck));
        sessionStorage.setItem('battleMode', mode);
        sessionStorage.removeItem('selectedPlayerDeck');
        sessionStorage.removeItem('selectedOpponentDeck');
        navigate(mode === 'player' ? '/online-lobby' : '/team-preview');
    };

    if (loading) {
        return (
            <div className="min-h-dvh bg-[var(--surface-1)] flex items-center justify-center">
                <div className="text-[var(--text-muted)] text-lg">読み込み中...</div>
            </div>
        );
    }

    return (
        <div className="min-h-dvh bg-[var(--surface-1)]">
            {/* Header */}
            <header className="bg-[var(--surface-2)] border-b border-[var(--border)] sticky top-0 z-20">
                <div className="max-w-5xl mx-auto px-6 py-4 flex items-center gap-4">
                    <button
                        onClick={() => navigate('/home')}
                        className="p-2 hover:bg-[var(--surface-3)] rounded-lg transition-colors"
                        aria-label="ホームに戻る"
                    >
                        <ArrowLeft className="size-5 text-[var(--text-muted)]" />
                    </button>
                    <div>
                        <h1 className="text-lg font-semibold text-[var(--text-primary)]">デッキ作成</h1>
                        <p className="text-sm text-[var(--text-muted)]">6匹のデッキを作成してください</p>
                    </div>
                </div>
            </header>

            <main className="max-w-5xl mx-auto px-6 py-8">
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
                    {/* Selected Pokemon */}
                    <div className="lg:col-span-1">
                        <div className="bg-[var(--surface-2)] border border-[var(--border)] rounded-xl p-5 sticky top-24">
                            <h2 className="text-base font-semibold text-[var(--text-primary)] mb-4">
                                選択中 <span className="text-[var(--text-muted)] font-normal">({selectedPokemon.length}/{DECK_SIZE})</span>
                            </h2>

                            <div className="space-y-3">
                            {Array.from({ length: DECK_SIZE }, (_, idx) => idx).map((idx) => (
                                    <div
                                        key={idx}
                                        className={`p-4 rounded-xl border transition-all ${selectedPokemon[idx]
                                            ? 'bg-[var(--accent-muted)] border-[var(--accent)]/30'
                                            : 'bg-[var(--surface-3)] border-[var(--border)] border-dashed'
                                            }`}
                                    >
                                        {selectedPokemon[idx] ? (
                                            <SelectedPokemonCard
                                                pokemon={selectedPokemon[idx]}
                                                species={species[selectedPokemon[idx].speciesId]}
                                                moves={moves}
                                                onRemove={() => handleRemovePokemon(idx)}
                                                onEditMoves={() => handleEditMoves(idx)}
                                                onEditEVs={() => handleEditEVs(idx)}
                                                onAbilityChange={(ability) => handleAbilityChange(idx, ability)}
                                            />
                                        ) : (
                                            <div className="text-center text-[var(--text-muted)] py-3">
                                                スロット {idx + 1}
                                            </div>
                                        )}
                                    </div>
                                ))}
                            </div>

                            <button
                                onClick={handleStartBattle}
                                disabled={selectedPokemon.length < DECK_SIZE}
                                className={`w-full mt-6 py-3.5 rounded-xl font-semibold flex items-center justify-center gap-2 transition-all
                                    ${selectedPokemon.length >= DECK_SIZE
                                        ? 'bg-[var(--accent)] text-white hover:bg-[var(--accent-hover)] shadow-lg shadow-[var(--accent)]/20'
                                        : 'bg-[var(--surface-3)] text-[var(--text-muted)] cursor-not-allowed'
                                    }`}
                            >
                                <Swords className="size-5" />
                                バトル開始
                            </button>
                        </div>
                    </div>

                    {/* Pokemon / Move Selection */}
                    <div className="lg:col-span-2">
                        {editingEVIndex !== null ? (
                            <EVEditor
                                species={species[selectedPokemon[editingEVIndex].speciesId]}
                                evs={editingEVs}
                                onEVChange={setEditingEVs}
                                onSave={handleSaveEVs}
                                onCancel={() => setEditingEVIndex(null)}
                            />
                        ) : editingIndex !== null ? (
                            <MoveSelector
                                species={species[selectedPokemon[editingIndex].speciesId]}
                                moves={moves}
                                learnsets={learnsets}
                                selectedMoves={editingMoves}
                                onToggleMove={handleToggleMove}
                                onSave={handleSaveMoves}
                                onCancel={() => setEditingIndex(null)}
                            />
                        ) : (
                            <div>
                                <h2 className="text-base font-semibold text-[var(--text-primary)] mb-4">ポケモン選択</h2>
                                <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                                    {speciesList.map((mon) => {
                                        const isSelected = selectedPokemon.some(p => p.speciesId === mon.id);
                                        return (
                                            <button
                                                key={mon.id}
                                                onClick={() => handleSelectPokemon(mon)}
                                                disabled={isSelected || selectedPokemon.length >= DECK_SIZE}
                                                className={`p-4 rounded-xl border text-left transition-all ${isSelected
                                                    ? 'bg-[var(--accent-muted)] border-[var(--accent)]/50'
                                                    : selectedPokemon.length >= DECK_SIZE
                                                        ? 'bg-[var(--surface-2)] border-[var(--border)] opacity-50 cursor-not-allowed'
                                                        : 'bg-[var(--surface-2)] border-[var(--border)] hover:border-[var(--border-hover)] hover:bg-[var(--surface-3)] card-hover'
                                                    }`}
                                            >
                                                <h3 className="font-semibold text-[var(--text-primary)]">{mon.name}</h3>
                                                <div className="flex gap-1 mt-2">
                                                    {mon.type.map((t) => (
                                                        <span
                                                            key={t}
                                                            className="px-2 py-0.5 text-xs text-white rounded-md"
                                                            style={{ backgroundColor: getTypeColor(t) }}
                                                        >
                                                            {t}
                                                        </span>
                                                    ))}
                                                </div>
                                                {isSelected && (
                                                    <div className="mt-2 flex items-center gap-1 text-xs text-[var(--accent)]">
                                                        <Check className="size-3" /> 選択済
                                                    </div>
                                                )}
                                            </button>
                                        );
                                    })}
                                </div>
                            </div>
                        )}
                    </div>
                </div>
            </main>
        </div>
    );
}

function sanitizeDeckPokemon(
    pokemon: DeckPokemon,
    species: SpeciesData,
    moves: MoveData,
    learnsets: Learnset,
): DeckPokemon | null {
    const mon = species[pokemon.speciesId];
    if (!mon) {
        return null;
    }

    const sanitizedMoves = pokemon.moves
        .filter((moveId, index, self) => self.indexOf(moveId) === index)
        .filter((moveId) => moves[moveId])
        .slice(0, 4);

    const fallbackMoves = (learnsets[pokemon.speciesId] || [])
        .filter((moveId) => moves[moveId])
        .slice(0, 4);

    return {
        ...pokemon,
        ability: mon.abilities.includes(pokemon.ability) ? pokemon.ability : (mon.abilities[0] || 'none'),
        moves: sanitizedMoves.length > 0 ? sanitizedMoves : fallbackMoves,
    };
}

function SelectedPokemonCard({
    pokemon,
    species,
    moves,
    onRemove,
    onEditMoves,
    onEditEVs,
    onAbilityChange,
}: {
    pokemon: DeckPokemon;
    species: Species;
    moves: MoveData;
    onRemove: () => void;
    onEditMoves: () => void;
    onEditEVs: () => void;
    onAbilityChange: (ability: string) => void;
}) {
    const totalEvs = pokemon.evs ? pokemon.evs.hp + pokemon.evs.atk + pokemon.evs.def + pokemon.evs.spa + pokemon.evs.spd + pokemon.evs.spe : 0;

    return (
        <div>
            <div className="flex items-center justify-between mb-2">
                <h3 className="font-semibold text-[var(--text-primary)]">{species.name}</h3>
                <button onClick={onRemove} className="p-1.5 hover:bg-red-500/20 rounded-lg transition-colors" aria-label="ポケモンを削除">
                    <X className="size-4 text-red-400" />
                </button>
            </div>
            <div className="flex gap-1 mb-3">
                {species.type.map((t) => (
                    <span
                        key={t}
                        className="px-2 py-0.5 text-xs text-white rounded-md"
                        style={{ backgroundColor: getTypeColor(t) }}
                    >
                        {t}
                    </span>
                ))}
            </div>
            <div className="mb-2">
                <label className="text-xs text-[var(--text-muted)]">特性</label>
                <select
                    value={pokemon.ability}
                    onChange={(e) => onAbilityChange(e.target.value)}
                    className="ml-2 text-xs bg-[var(--surface-3)] text-[var(--text-primary)] rounded p-1"
                >
                    {species.abilities.map((abilityId) => (
                        <option key={abilityId} value={abilityId}>
                            {getAbilityLabel(abilityId)}
                        </option>
                    ))}
                </select>
            </div>
            <div className="space-y-1">
                {pokemon.moves.map((moveId) => {
                    const move = moves[moveId];
                    return (
                        <div key={moveId} className="text-xs text-[var(--text-secondary)] flex items-center gap-2">
                            <span
                                className="size-2 rounded-full"
                                style={{ backgroundColor: getTypeColor(move?.type || 'normal') }}
                            />
                            {move?.name || moveId}
                        </div>
                    );
                })}
            </div>
            {totalEvs > 0 && (
                <div className="mt-2 text-xs text-[var(--text-muted)]">
                    EV: {totalEvs}/510
                </div>
            )}
            <div className="flex gap-2 mt-3">
                <button
                    onClick={onEditMoves}
                    className="text-xs text-[var(--accent)] hover:text-[var(--accent-hover)] transition-colors"
                >
                    技を編集
                </button>
                <button
                    onClick={onEditEVs}
                    className="text-xs text-[var(--accent)] hover:text-[var(--accent-hover)] transition-colors flex items-center gap-1"
                >
                    <Sliders className="size-3" />
                    EVを編集
                </button>
            </div>
        </div>
    );
}

function MoveSelector({
    species,
    moves,
    learnsets,
    selectedMoves,
    onToggleMove,
    onSave,
    onCancel,
}: {
    species: Species;
    moves: MoveData;
    learnsets: Learnset;
    selectedMoves: string[];
    onToggleMove: (moveId: string) => void;
    onSave: () => void;
    onCancel: () => void;
}) {
    const availableMoves = (learnsets[species.id] || []).filter(m => moves[m]);

    return (
        <div>
            <div className="flex items-center justify-between mb-5">
                <h2 className="text-base font-semibold text-[var(--text-primary)]">
                    {species.name}の技を選択 <span className="text-[var(--text-muted)] font-normal">({selectedMoves.length}/4)</span>
                </h2>
                <div className="flex gap-2">
                    <button
                        onClick={onCancel}
                        className="px-4 py-2 bg-[var(--surface-3)] text-[var(--text-primary)] rounded-lg hover:bg-[var(--surface-4)] transition-colors"
                    >
                        キャンセル
                    </button>
                    <button
                        onClick={onSave}
                        className="px-4 py-2 bg-[var(--accent)] text-white rounded-lg hover:bg-[var(--accent-hover)] transition-colors"
                    >
                        保存
                    </button>
                </div>
            </div>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {availableMoves.map((moveId) => {
                    const move = moves[moveId];
                    const isSelected = selectedMoves.includes(moveId);
                    return (
                        <button
                            key={moveId}
                            onClick={() => onToggleMove(moveId)}
                            disabled={!isSelected && selectedMoves.length >= 4}
                            className={`p-4 rounded-xl border text-left transition-all ${isSelected
                                ? 'bg-[var(--accent-muted)] border-[var(--accent)]/50'
                                : selectedMoves.length >= 4
                                    ? 'bg-[var(--surface-2)] border-[var(--border)] opacity-50 cursor-not-allowed'
                                    : 'bg-[var(--surface-2)] border-[var(--border)] hover:border-[var(--border-hover)] hover:bg-[var(--surface-3)]'
                                }`}
                        >
                            <div className="flex items-center gap-2">
                                <span
                                    className="px-2 py-0.5 text-xs text-white rounded-md"
                                    style={{ backgroundColor: getTypeColor(move.type) }}
                                >
                                    {move.type}
                                </span>
                                <span className="font-medium text-[var(--text-primary)]">{move.name}</span>
                                {isSelected && <Check className="size-4 text-[var(--accent)] ml-auto" />}
                            </div>
                            <div className="mt-2 text-xs text-[var(--text-muted)] flex gap-3 tabular-nums">
                                <span>威力: {move.power || '-'}</span>
                                <span>PP: {move.pp}</span>
                                <span>{move.category}</span>
                            </div>
                        </button>
                    );
                })}
            </div>
        </div>
    );
}

function EVEditor({
    species,
    evs,
    onEVChange,
    onSave,
    onCancel,
}: {
    species: Species;
    evs: EVStats;
    onEVChange: (evs: EVStats) => void;
    onSave: () => void;
    onCancel: () => void;
}) {
    const stats = ['hp', 'atk', 'def', 'spa', 'spd', 'spe'] as const;
    const statLabels = { hp: 'HP', atk: '攻撃', def: '防御', spa: '特攻', spd: '特防', spe: '素早' };
    const total = evs.hp + evs.atk + evs.def + evs.spa + evs.spd + evs.spe;
    const remaining = 510 - total;

    const handleStatChange = (stat: keyof EVStats, value: number) => {
        const current = evs[stat];
        const newValue = Math.min(252, Math.max(0, value));
        const diff = newValue - current;

        if (total + diff > 510) {
            const maxAllowed = Math.min(252, current + remaining);
            onEVChange({ ...evs, [stat]: maxAllowed });
        } else {
            onEVChange({ ...evs, [stat]: newValue });
        }
    };

    return (
        <div>
            <div className="flex items-center justify-between mb-5">
                <h2 className="text-base font-semibold text-[var(--text-primary)]">
                    {species.name}のEVを編集
                    <span className="text-[var(--text-muted)] font-normal ml-2">({total}/510)</span>
                </h2>
                <div className="flex gap-2">
                    <button
                        onClick={onCancel}
                        className="px-4 py-2 bg-[var(--surface-3)] text-[var(--text-primary)] rounded-lg hover:bg-[var(--surface-4)] transition-colors"
                    >
                        キャンセル
                    </button>
                    <button
                        onClick={onSave}
                        className="px-4 py-2 bg-[var(--accent)] text-white rounded-lg hover:bg-[var(--accent-hover)] transition-colors"
                    >
                        保存
                    </button>
                </div>
            </div>

            {remaining < 0 && (
                <div className="mb-4 p-3 bg-red-500/10 border border-red-500/30 rounded-lg text-red-400 text-sm">
                    ⚠️ 合計が510を超えています
                </div>
            )}

            <div className="bg-[var(--surface-2)] border border-[var(--border)] rounded-xl p-5 space-y-5">
                {stats.map((stat) => (
                    <div key={stat} className="space-y-2">
                        <div className="flex items-center justify-between">
                            <span className="text-sm font-medium text-[var(--text-primary)]">{statLabels[stat]}</span>
                            <div className="flex items-center gap-2">
                                <input
                                    type="number"
                                    min={0}
                                    max={252}
                                    value={evs[stat]}
                                    onChange={(e) => handleStatChange(stat, parseInt(e.target.value) || 0)}
                                    className="w-16 px-2 py-1 bg-[var(--surface-3)] border border-[var(--border)] rounded text-center text-sm text-[var(--text-primary)] tabular-nums"
                                />
                                <span className="text-xs text-[var(--text-muted)] w-10">/ 252</span>
                            </div>
                        </div>
                        <input
                            type="range"
                            min={0}
                            max={252}
                            step={4}
                            value={evs[stat]}
                            onChange={(e) => handleStatChange(stat, parseInt(e.target.value))}
                            className="w-full accent-[var(--accent)]"
                        />
                    </div>
                ))}
            </div>

            <div className="mt-4 flex justify-center gap-2">
                <button
                    onClick={() => onEVChange({ hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 })}
                    className="px-3 py-1.5 bg-[var(--surface-3)] text-[var(--text-muted)] rounded-lg text-sm hover:bg-[var(--surface-4)] transition-colors"
                >
                    リセット
                </button>
            </div>
        </div>
    );
}
