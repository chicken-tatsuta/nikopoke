import { useState, useEffect, useRef, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { cn } from '../lib/cn';
import { loadAllData, getTypeColor } from '../lib/data';
import { BattleLog, ActionSummary } from '../components/BattleLog';
import { getAbilityLabel } from './PokemonDetailPage';
import { createBattleStatsId, uploadGlobalBattleRecord } from '../lib/globalBattleStats';
import {
    initEngine,
    createBattleState,
    stepBattle,
    getFirstAvailableSwitchSlot,
    getBestMoveMinimax,
    isBattleOver,
    getWinner,
    needsForcedSwitch,
    replaceFaintedPokemon,
    type BattleStateWire,
    type PlayerStateWire,
    type CreatureStateWire,
    type ActionWire
} from '../lib/engine';
import {
    clearOnlineSession,
    getOnlineSessionSnapshot,
    sendBattleInit,
    sendBattleUpdate,
    sendPlayerAction,
    subscribeOnlineSession,
    type OnlineRole,
} from '../lib/p2p';
import type { SpeciesData, MoveData, DeckPokemon } from '../types/pokemon';
type FieldEffectValue =
    | boolean
    | number
    | string
    | null
    | undefined
    | {
        id?: string;
        name?: string;
        active?: boolean;
        turns?: number;
        remaining?: number;
        duration?: number;
        layers?: number;
        [key: string]: unknown;
    };

type BattleFieldLike = {
    global?: Record<string, FieldEffectValue>;
    sides?: Array<Record<string, FieldEffectValue>> | Record<string, Record<string, FieldEffectValue>>;
};

type BattleStateWithField = BattleStateWire & {
    field?: BattleFieldLike;
};

type FieldEffectItem = {
    key: string;
    label: string;
    turns?: number;
    layers?: number;
};

const FIELD_EFFECT_LABELS: Record<string, string> = {
    sun: '晴れ',
    sunny: '晴れ',
    harsh_sunlight: '晴れ',
    rain: '雨',
    rainy: '雨',
    sandstorm: '砂嵐',
    hail: 'あられ',
    snow: '雪',

    electric_terrain: 'エレキフィールド',
    grassy_terrain: 'グラスフィールド',
    misty_terrain: 'ミストフィールド',
    psychic_terrain: 'サイコフィールド',

    stealth_rock: 'ステルスロック',
    spikes: 'まきびし',
    toxic_spikes: 'どくびし',
    sticky_web: 'ねばねばネット',

    reflect: 'リフレクター',
    light_screen: 'ひかりのかべ',
    aurora_veil: 'オーロラベール',
    safeguard: 'しんぴのまもり',
    tailwind: 'おいかぜ',
};

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

const TYPE_EFFECTIVENESS: Record<string, Partial<Record<string, number>>> = {
    normal: {
      rock: 0.5,
      ghost: 0,
      steel: 0.5,
    },
    fire: {
      fire: 0.5,
      water: 0.5,
      grass: 2,
      ice: 2,
      bug: 2,
      rock: 0.5,
      dragon: 0.5,
      steel: 2,
    },
    water: {
      fire: 2,
      water: 0.5,
      grass: 0.5,
      ground: 2,
      rock: 2,
      dragon: 0.5,
    },
    electric: {
      water: 2,
      electric: 0.5,
      grass: 0.5,
      ground: 0,
      flying: 2,
      dragon: 0.5,
    },
    grass: {
      fire: 0.5,
      water: 2,
      grass: 0.5,
      poison: 0.5,
      ground: 2,
      flying: 0.5,
      bug: 0.5,
      rock: 2,
      dragon: 0.5,
      steel: 0.5,
    },
    ice: {
      fire: 0.5,
      water: 0.5,
      grass: 2,
      ice: 0.5,
      ground: 2,
      flying: 2,
      dragon: 2,
      steel: 0.5,
    },
    fighting: {
      normal: 2,
      ice: 2,
      poison: 0.5,
      flying: 0.5,
      psychic: 0.5,
      bug: 0.5,
      rock: 2,
      ghost: 0,
      dark: 2,
      steel: 2,
      fairy: 0.5,
    },
    poison: {
      grass: 2,
      poison: 0.5,
      ground: 0.5,
      rock: 0.5,
      ghost: 0.5,
      steel: 0,
      fairy: 2,
    },
    ground: {
      fire: 2,
      electric: 2,
      grass: 0.5,
      poison: 2,
      flying: 0,
      bug: 0.5,
      rock: 2,
      steel: 2,
    },
    flying: {
      electric: 0.5,
      grass: 2,
      fighting: 2,
      bug: 2,
      rock: 0.5,
      steel: 0.5,
    },
    psychic: {
      fighting: 2,
      poison: 2,
      psychic: 0.5,
      dark: 0,
      steel: 0.5,
    },
    bug: {
      fire: 0.5,
      grass: 2,
      fighting: 0.5,
      poison: 0.5,
      flying: 0.5,
      psychic: 2,
      ghost: 0.5,
      dark: 2,
      steel: 0.5,
      fairy: 0.5,
    },
    rock: {
      fire: 2,
      ice: 2,
      fighting: 0.5,
      ground: 0.5,
      flying: 2,
      bug: 2,
      steel: 0.5,
    },
    ghost: {
      normal: 0,
      psychic: 2,
      ghost: 2,
      dark: 0.5,
    },
    dragon: {
      dragon: 2,
      steel: 0.5,
      fairy: 0,
    },
    dark: {
      fighting: 0.5,
      psychic: 2,
      ghost: 2,
      dark: 0.5,
      fairy: 0.5,
    },
    steel: {
      fire: 0.5,
      water: 0.5,
      electric: 0.5,
      ice: 2,
      rock: 2,
      steel: 0.5,
      fairy: 2,
    },
    fairy: {
      fire: 0.5,
      fighting: 2,
      poison: 0.5,
      dragon: 2,
      dark: 2,
      steel: 0.5,
    },
  };

  function getTypeEffectiveness(moveType?: string, targetTypes?: string[]): number | null {
    if (!moveType || !targetTypes?.length) return null;
  
    return targetTypes.reduce((multiplier, targetType) => {
      const typeMultiplier = TYPE_EFFECTIVENESS[moveType]?.[targetType] ?? 1;
      return multiplier * typeMultiplier;
    }, 1);
  }
  
  function getEffectivenessLabel(multiplier: number | null): string | null {
    if (multiplier === null) return null;
    if (multiplier === 0) return '無効';
    if (multiplier >= 4) return 'ちょうばつぐん';
    if (multiplier >= 2) return 'ばつぐん';
    if (multiplier <= 0.25) return 'かなりいまひとつ';
    if (multiplier < 1) return 'いまひとつ';
    return null;
  }
  
  function getEffectivenessClass(multiplier: number | null): string {
    if (multiplier === 0) {
      return 'bg-slate-800 text-white border border-slate-700';
    }
  
    if (multiplier !== null && multiplier >= 4) {
      return 'bg-pink-100 text-pink-700 border border-pink-200';
    }
  
    if (multiplier !== null && multiplier >= 2) {
      return 'bg-red-100 text-red-700 border border-red-200';
    }
  
    if (multiplier !== null && multiplier <= 0.25) {
      return 'bg-indigo-100 text-indigo-700 border border-indigo-200';
    }
  
    if (multiplier !== null && multiplier < 1) {
      return 'bg-blue-100 text-blue-700 border border-blue-200';
    }
  
    return 'bg-slate-100 text-slate-500 border border-slate-200';
  }
  
  function formatEffectivenessMultiplier(multiplier: number | null): string {
    if (multiplier === null || multiplier === 1) return '';
    return ` ×${multiplier}`;
  }

function getEffectLabel(key: string): string {
    return FIELD_EFFECT_LABELS[key] ?? key.replace(/^field_/, '').replace(/^side_/, '').replace(/_/g, ' ');
}

function isActiveEffect(value: FieldEffectValue): boolean {
    if (value == null) return false;
    if (typeof value === 'boolean') return value;
    if (typeof value === 'number') return value > 0;
    if (typeof value === 'string') return value.length > 0;

    if (value.active === false) return false;
    if (typeof value.remaining === 'number' && value.remaining <= 0) return false;
    if (typeof value.turns === 'number' && value.turns <= 0) return false;
    if (typeof value.layers === 'number' && value.layers <= 0) return false;

    return true;
}

function normalizeEffects(effects?: Record<string, FieldEffectValue>): FieldEffectItem[] {
    if (!effects) return [];

    return Object.entries(effects)
        .filter(([, value]) => isActiveEffect(value))
        .map(([key, value]) => {
            if (typeof value === 'object' && value !== null) {
                const effectKey = value.id ?? key;

                return {
                    key: effectKey,
                    label: value.name ?? getEffectLabel(effectKey),
                    turns:
                        typeof value.remaining === 'number'
                            ? value.remaining
                            : typeof value.turns === 'number'
                                ? value.turns
                                : undefined,
                    layers: typeof value.layers === 'number' ? value.layers : undefined,
                };
            }

            if (typeof value === 'string') {
                return {
                    key: value,
                    label: getEffectLabel(value),
                };
            }

            return {
                key,
                label: getEffectLabel(key),
                layers:
                    typeof value === 'number' && ['spikes', 'toxic_spikes'].includes(key)
                        ? value
                        : undefined,
            };
        });
}

function getSideField(
    sides: BattleFieldLike['sides'],
    playerId: string,
    fallbackIndex: number,
): Record<string, FieldEffectValue> | undefined {
    if (!sides) return undefined;

    if (Array.isArray(sides)) {
        return sides[fallbackIndex];
    }

    return sides[playerId];
}

function FieldEffectChip({ effect }: { effect: FieldEffectItem }) {
    const details = [
        effect.layers && effect.layers > 1 ? `${effect.layers}層` : null,
        effect.turns ? `あと${effect.turns}T` : null,
    ]
        .filter(Boolean)
        .join(' / ');

    return (
        <span className="inline-flex items-center gap-1 rounded-full border border-[var(--border)] bg-[var(--surface-3)] px-2 py-1 text-xs text-[var(--text-primary)]">
            <span>{effect.label}</span>
            {details && <span className="text-[var(--text-muted)]">{details}</span>}
        </span>
    );
}

function EmptyFieldEffect() {
    return <span className="text-xs text-[var(--text-muted)]">なし</span>;
}

function BattleFieldStatusPanel({
    field,
    localPlayerId,
    opponentPlayerId,
}: {
    field?: BattleFieldLike;
    localPlayerId: string;
    opponentPlayerId: string;
}) {
    const globalEffects = normalizeEffects(field?.global);
    const opponentSideEffects = normalizeEffects(getSideField(field?.sides, opponentPlayerId, 1));
    const playerSideEffects = normalizeEffects(getSideField(field?.sides, localPlayerId, 0));

    return (
        <section className="rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-3">
            <div className="mb-2 text-sm font-bold text-[var(--text-primary)]">場の状態</div>

            <div className="space-y-3">
                <div>
                    <div className="mb-1 text-xs text-[var(--text-muted)]">天候・フィールド</div>
                    <div className="flex flex-wrap gap-1.5">
                        {globalEffects.length > 0 ? (
                            globalEffects.map((effect) => (
                                <FieldEffectChip key={effect.key} effect={effect} />
                            ))
                        ) : (
                            <EmptyFieldEffect />
                        )}
                    </div>
                </div>

                <div className="grid grid-cols-2 gap-2">
                    <div className="rounded-lg border border-[var(--border)] bg-[var(--surface-3)] p-2">
                        <div className="mb-1 text-xs text-[var(--text-muted)]">相手側</div>
                        <div className="flex flex-wrap gap-1.5">
                            {opponentSideEffects.length > 0 ? (
                                opponentSideEffects.map((effect) => (
                                    <FieldEffectChip key={effect.key} effect={effect} />
                                ))
                            ) : (
                                <EmptyFieldEffect />
                            )}
                        </div>
                    </div>

                    <div className="rounded-lg border border-[var(--border)] bg-[var(--surface-3)] p-2">
                        <div className="mb-1 text-xs text-[var(--text-muted)]">自分側</div>
                        <div className="flex flex-wrap gap-1.5">
                            {playerSideEffects.length > 0 ? (
                                playerSideEffects.map((effect) => (
                                    <FieldEffectChip key={effect.key} effect={effect} />
                                ))
                            ) : (
                                <EmptyFieldEffect />
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}
const STATUS_LABELS: Record<string, string> = {
    sleep: 'ねむり',
    asleep: 'ねむり',
    burn: 'やけど',
    burned: 'やけど',
    poison: 'どく',
    poisoned: 'どく',
    toxic: 'もうどく',
    badly_poison: 'もうどく',
    badly_poisoned: 'もうどく',
    paralysis: 'まひ',
    paralyzed: 'まひ',
    paralyze: 'まひ',
    freeze: 'こおり',
    frozen: 'こおり',
    confusion: 'こんらん',
    confused: 'こんらん',
    flinch: 'ひるみ',
    faint: 'ひんし',
    fainted: 'ひんし',

    leech_seed: 'やどりぎ',
    substitute: 'みがわり',
    protect: 'まもる',
    endure: 'こらえる',
    taunt: 'ちょうはつ',
    encore: 'アンコール',
    disable: 'かなしばり',
};

function getStatusLabel(statusId: string): string {
    return STATUS_LABELS[statusId] ?? statusId.replace(/_/g, ' ');
}



export default function BattlePage() {
    const navigate = useNavigate();
    const [battleMode] = useState<'ai' | 'player'>(() =>
        sessionStorage.getItem('battleMode') === 'player' ? 'player' : 'ai',
    );
    const [species, setSpecies] = useState<SpeciesData>({});
    const [moves, setMoves] = useState<MoveData>({});
    const [battleState, setBattleState] = useState<BattleStateWire | null>(null);
    const [loading, setLoading] = useState(true);
    const [waiting, setWaiting] = useState(false);    
    const [commandMode, setCommandMode] = useState<'fight' | 'pokemon'>('fight');
    const [focusedTeamSlot, setFocusedTeamSlot] = useState(0);
    const [lastMoves, setLastMoves] = useState<{ player?: string; ai?: string }>({});
    const [onlineSnapshot, setOnlineSnapshot] = useState(getOnlineSessionSnapshot());
    const [localPlayerId, setLocalPlayerId] = useState<string>('player');
    const [opponentPlayerId, setOpponentPlayerId] = useState<string>('ai');
    const [statusText, setStatusText] = useState('');
    const logsRef = useRef<HTMLDivElement>(null);
    const battleStateRef = useRef<BattleStateWire | null>(null);
    const localPlayerIdRef = useRef(localPlayerId);
    const opponentPlayerIdRef = useRef(opponentPlayerId);
    const localDeckRef = useRef<DeckPokemon[] | null>(null);
    const opponentDeckRef = useRef<DeckPokemon[] | null>(null);
    const battleRecordSavedRef = useRef(false);
    const battleStatsIdRef = useRef<string>(createBattleStatsId());
    const onlineRoleRef = useRef<OnlineRole | null>(onlineSnapshot.role);
    const pendingLocalActionRef = useRef<ActionWire | null>(null);
    const pendingRemoteActionRef = useRef<ActionWire | null>(null);
    const resolvingTurnRef = useRef(false);
    const initializedRef = useRef(false);

    useEffect(() => {
        battleStateRef.current = battleState;
    }, [battleState]);

    useEffect(() => {
        localPlayerIdRef.current = localPlayerId;
    }, [localPlayerId]);

    useEffect(() => {
        opponentPlayerIdRef.current = opponentPlayerId;
    }, [opponentPlayerId]);

    useEffect(() => {
        onlineRoleRef.current = onlineSnapshot.role;
    }, [onlineSnapshot.role]);

    const updateLastMovesFromActions = useCallback((actions: ActionWire[]) => {
        const localId = localPlayerIdRef.current;
        const opponentId = opponentPlayerIdRef.current;
        setLastMoves({
            player: actions.find((action) => action.playerId === localId)?.moveId,
            ai: actions.find((action) => action.playerId === opponentId)?.moveId,
        });
    }, []);

    const finishBattle = useCallback(async (nextState: BattleStateWire) => {
        const over = await isBattleOver(nextState);
    
        if (!over) {
            return false;
        }
    
        const winner = getWinner(nextState);

        const winnerSide =
            winner === localPlayerIdRef.current
                ? 'player'
                : winner === opponentPlayerIdRef.current
                  ? 'opponent'
                  : null;
        const shouldUploadStats =
            !battleRecordSavedRef.current &&
            winnerSide &&
            localDeckRef.current &&
            opponentDeckRef.current &&
            (battleMode === 'ai' || localPlayerIdRef.current === 'host');

        if (shouldUploadStats) {
            battleRecordSavedRef.current = true;

            await uploadGlobalBattleRecord({
                id: battleStatsIdRef.current,
                winner: winnerSide,
                playerDeck: localDeckRef.current,
                opponentDeck: opponentDeckRef.current,
                mode: battleMode,
            });
        }

        const resultPayload = {
            winner,
            localPlayerId: localPlayerIdRef.current,
            logs: nextState.log,
        };

        window.setTimeout(() => {
            navigate('/result', {
                state: {
                    battleMode,
                    result: resultPayload,
                },
            });
        }, 1500);

        return true;
    }, [battleMode, navigate]);

    const resetBattlePersistenceState = useCallback(() => {
        battleRecordSavedRef.current = false;
        battleStatsIdRef.current = createBattleStatsId();
    }, []);

    const resolveHostTurn = useCallback(async (localAction: ActionWire, remoteAction: ActionWire) => {
        const currentState = battleStateRef.current;
        if (!currentState || resolvingTurnRef.current) {
            return;
        }
        resolvingTurnRef.current = true;
        try {
            const actions = [localAction, remoteAction];
            const nextState = await stepBattle(currentState, actions);
            pendingLocalActionRef.current = null;
            pendingRemoteActionRef.current = null;
            updateLastMovesFromActions(actions);
            setBattleState(nextState);
            sendBattleUpdate(nextState, actions);
            const finished = await finishBattle(nextState);
            if (!finished) {
                setWaiting(false);
                setStatusText('');
            }
        } catch (error) {
            console.error('Online battle step error:', error);
            setStatusText('ターンの解決に失敗しました。');
            setWaiting(false);
        } finally {
            resolvingTurnRef.current = false;
        }
    }, [finishBattle, updateLastMovesFromActions]);

    const resolveForcedSwitch = useCallback(async (action: ActionWire, broadcast: boolean) => {
        const currentState = battleStateRef.current;
        if (!currentState || action.type !== 'switch' || typeof action.slot !== 'number') {
            return;
        }

        try {
            const nextState = replaceFaintedPokemon(currentState, action.playerId, action.slot);
            pendingLocalActionRef.current = null;
            pendingRemoteActionRef.current = null;
            setBattleState(nextState);
            if (broadcast) {
                sendBattleUpdate(nextState, [action]);
            }
            const finished = await finishBattle(nextState);
            if (!finished) {
                setWaiting(false);
                setStatusText('');
            }
        } catch (error) {
            console.error('Forced switch error:', error);
            setStatusText('ポケモンの出し直しに失敗しました。');
            setWaiting(false);
        }
    }, [finishBattle]);

    useEffect(() => {
        let cancelled = false;
        const boot = async () => {
            await initEngine();
            const { species: loadedSpecies, moves: loadedMoves } = await loadAllData();
            if (cancelled) {
                return;
            }
            setSpecies(loadedSpecies);
            setMoves(loadedMoves);
            setLoading(false);
        };

        boot().catch((error) => {
            console.error('Failed to initialize battle data:', error);
            if (!cancelled) {
                setStatusText('バトル準備に失敗しました。');
                setLoading(false);
            }
        });

        return () => {
            cancelled = true;
        };
    }, [battleMode, navigate]);

    useEffect(() => {
        if (logsRef.current) {
            logsRef.current.scrollTop = logsRef.current.scrollHeight;
        }
    }, [battleState?.log]);

    useEffect(() => {
        if (battleMode !== 'player') {
            return;
        }

        return subscribeOnlineSession((event) => {
            if (event.type === 'snapshot') {
                setOnlineSnapshot(event.snapshot);
                return;
            }
            if (event.type === 'battle_init') {
                setBattleState(event.state);
                setWaiting(false);
                setStatusText('');
                return;
            }
            if (event.type === 'battle_update') {
                updateLastMovesFromActions(event.actions);
                setBattleState(event.state);
                setWaiting(false);
                setStatusText('');
                void finishBattle(event.state);
                return;
            }
            if (event.type === 'remote_action' && onlineRoleRef.current === 'host') {
                const currentState = battleStateRef.current;
                if (
                    currentState &&
                    event.action.type === 'switch' &&
                    needsForcedSwitch(currentState, event.action.playerId)
                ) {
                    void resolveForcedSwitch(event.action, true);
                    return;
                }
                const localAction = pendingLocalActionRef.current;
                if (localAction) {
                    void resolveHostTurn(localAction, event.action);
                    return;
                }
                pendingRemoteActionRef.current = event.action;
                setStatusText('相手の入力を受け取りました。あなたの行動を選んでください。');
                return;
            }
            if (event.type === 'peer_left') {
                setStatusText('相手との接続が切れました。');
                setWaiting(false);
                return;
            }
            if (event.type === 'error') {
                setStatusText(event.message);
                setWaiting(false);
            }
        });
    }, [battleMode, finishBattle, resolveForcedSwitch, resolveHostTurn, updateLastMovesFromActions]);

    useEffect(() => {
        if (loading || initializedRef.current) {
            return;
        }

        const deckJson = sessionStorage.getItem('playerDeck');
        if (!deckJson) {
            navigate('/home');
            return;
        }

        if (battleMode === 'ai') {
            const selectedPlayerDeckJson = sessionStorage.getItem('selectedPlayerDeck');
            const selectedOpponentDeckJson = sessionStorage.getItem('selectedOpponentDeck');
        
            if (!selectedPlayerDeckJson || !selectedOpponentDeckJson) {
                navigate('/team-preview');
                return;
            }
        
            initializedRef.current = true;
        
            const playerDeck: DeckPokemon[] = JSON.parse(selectedPlayerDeckJson);
            const aiDeck: DeckPokemon[] = JSON.parse(selectedOpponentDeckJson);
        
            if (playerDeck.length !== 3 || aiDeck.length !== 3) {
                navigate('/team-preview');
                return;
            }
        
            localDeckRef.current = playerDeck;
            opponentDeckRef.current = aiDeck;
            resetBattlePersistenceState();

            createBattleState({
                player: { team: playerDeck },
                ai: { team: aiDeck },
            })
                .then((state) => {
                    setLocalPlayerId('player');
                    setOpponentPlayerId('ai');
                    setBattleState(state);
                })
                .catch((error) => {
                    console.error('Failed to create AI battle state:', error);
                    setStatusText('AI対戦の初期化に失敗しました。');
                });
            return;
        }

        if (!onlineSnapshot.role || !onlineSnapshot.localDeck) {
            navigate('/online-lobby');
            return;
        }
        const selectedPlayerDeckJson = sessionStorage.getItem('selectedPlayerDeck');
        const selectedOpponentDeckJson = sessionStorage.getItem('selectedOpponentDeck');

        if (!selectedPlayerDeckJson || !selectedOpponentDeckJson) {
            navigate('/team-preview');
            return;
        }

        const selectedPlayerDeck: DeckPokemon[] = JSON.parse(selectedPlayerDeckJson);
        const selectedOpponentDeck: DeckPokemon[] = JSON.parse(selectedOpponentDeckJson);

        if (selectedPlayerDeck.length !== 3 || selectedOpponentDeck.length !== 3) {
            navigate('/team-preview');
            return;
        }

        if (onlineSnapshot.role === 'host' && onlineSnapshot.remoteDeck) {
            initializedRef.current = true;
            setLocalPlayerId('host');
            setOpponentPlayerId('guest');
            localDeckRef.current = selectedPlayerDeck;
            opponentDeckRef.current = selectedOpponentDeck;
            resetBattlePersistenceState();
            createBattleState({
                host: { team: selectedPlayerDeck },
                guest: { team: selectedOpponentDeck },
            })
                .then((state) => {
                    setBattleState(state);
                    sendBattleInit(state);
                })
                .catch((error) => {
                    console.error('Failed to create online battle state:', error);
                    const message = error instanceof Error ? error.message : String(error);
                    setStatusText(`オンライン対戦の初期化に失敗しました: ${message}`);
                });
            return;
        }

        if (onlineSnapshot.role === 'guest') {
            initializedRef.current = true;
            localDeckRef.current = selectedPlayerDeck;
            opponentDeckRef.current = selectedOpponentDeck;
            resetBattlePersistenceState();
            setLocalPlayerId('guest');
            setOpponentPlayerId('host');
            if (onlineSnapshot.latestState) {
                setBattleState(onlineSnapshot.latestState);
            } else {
                setStatusText('ホストが対戦を開始するのを待っています...');
            }
        }
    }, [battleMode, loading, navigate, onlineSnapshot.localDeck, onlineSnapshot.latestState, onlineSnapshot.remoteDeck, onlineSnapshot.role, resetBattlePersistenceState, species]);

    useEffect(() => {
        if (battleMode !== 'ai' || waiting || !battleState) {
            return;
        }

        if (!needsForcedSwitch(battleState, opponentPlayerIdRef.current)) {
            return;
        }

        const slot = getFirstAvailableSwitchSlot(battleState, opponentPlayerIdRef.current);
        if (slot === null) {
            void finishBattle(battleState);
            return;
        }

        const nextState = replaceFaintedPokemon(battleState, opponentPlayerIdRef.current, slot);
        setBattleState(nextState);
        setWaiting(false);
        void finishBattle(nextState);
    }, [battleMode, battleState, finishBattle, waiting]);

    const getPlayer = (id: string): PlayerStateWire | undefined => {
        return battleState?.players.find(p => p.id === id);
    };

    const getFallbackAiAction = (state: BattleStateWire): ActionWire | null => {
        const aiPlayer = state.players.find((player) => player.id === opponentPlayerIdRef.current);
        if (!aiPlayer) return null;
    
        const activePokemon = aiPlayer.team[aiPlayer.activeSlot];
        if (!activePokemon) return null;
    
        const moveIds = activePokemon.moves ?? [];
        const movePp = activePokemon.movePp as unknown;
    
        const getPp = (moveId: string): number | undefined => {
            if (movePp instanceof Map) {
                return movePp.get(moveId);
            }
    
            if (movePp && typeof movePp === 'object') {
                return (movePp as Record<string, number | undefined>)[moveId];
            }
    
            return undefined;
        };
    
        const fallbackMoveId = moveIds.find((moveId) => {
            const pp = getPp(moveId);
            return pp === undefined || pp > 0;
        });
    
        if (!fallbackMoveId) return null;
    
        console.warn('[battle] AI minimax failed. fallback move selected:', {
            speciesId: activePokemon.speciesId,
            fallbackMoveId,
            moves: moveIds,
            movePp,
        });
    
        return {
            type: 'move',
            playerId: opponentPlayerIdRef.current,
            moveId: fallbackMoveId,
            targetId: localPlayerIdRef.current,
        };
    };
    const submitOnlineAction = async (action: ActionWire) => {
        if (onlineSnapshot.role === 'guest') {
            sendPlayerAction(action);
            setWaiting(true);
            setStatusText('ホストがターンを処理しています...');
            return;
        }

        const remoteAction = pendingRemoteActionRef.current;
        if (remoteAction) {
            setWaiting(true);
            await resolveHostTurn(action, remoteAction);
            return;
        }

        pendingLocalActionRef.current = action;
        setWaiting(true);
        setStatusText('相手の行動を待っています...');
    };

    const handleSelectMove = async (moveId: string) => {
        if (!battleState || waiting) return;
        setWaiting(true);
        setCommandMode('fight');

        if (battleState && needsForcedSwitch(battleState, localPlayerIdRef.current)) {
            setWaiting(false);
            return;
        }

        try {
            const playerAction: ActionWire = {
                type: 'move',
                playerId: localPlayerIdRef.current,
                moveId,
                targetId: opponentPlayerIdRef.current,
            };

            if (battleMode === 'player') {
                await submitOnlineAction(playerAction);
                return;
            }

            const aiAction = await getBestMoveMinimax(battleState, 'ai', 1) ?? getFallbackAiAction(battleState);

if (!aiAction) {
    console.error('AI failed to select action');
    setWaiting(false);
    return;
}

            const newState = await stepBattle(battleState, [playerAction, aiAction]);
            setLastMoves({
                player: moveId,
                ai: aiAction.moveId || undefined
            });
            setBattleState(newState);
            await finishBattle(newState);
        } catch (err) {
            console.error('Battle step error:', err);
            setStatusText('行動の送信に失敗しました。');
        }

        setWaiting(false);
    };

    const handleSwitch = async (index: number) => {
        if (!battleState || waiting) return;
        const player = getPlayer(localPlayerIdRef.current);
        if (!player) return;
        if (index === player.activeSlot) return;
        if (player.team[index].hp <= 0) return;

        setWaiting(true);
        setCommandMode('fight');

        try {
            const playerAction: ActionWire = {
                type: 'switch',
                playerId: localPlayerIdRef.current,
                slot: index
            };
            const forcedSwitch = needsForcedSwitch(battleState, localPlayerIdRef.current);

            if (battleMode === 'player') {
                if (forcedSwitch) {
                    if (onlineSnapshot.role === 'host') {
                        await resolveForcedSwitch(playerAction, true);
                    } else {
                        sendPlayerAction(playerAction);
                        setWaiting(true);
                        setStatusText('ホストがポケモンの出し直しを処理しています...');
                    }
                    return;
                }
                await submitOnlineAction(playerAction);
                return;
            }

            if (forcedSwitch) {
                const newState = replaceFaintedPokemon(battleState, localPlayerIdRef.current, index);
                setBattleState(newState);
                await finishBattle(newState);
                setWaiting(false);
                setStatusText('');
                return;
            }

            const aiAction = await getBestMoveMinimax(battleState, 'ai', 1) ?? getFallbackAiAction(battleState);

if (!aiAction) {
    console.error('AI failed to select action');
    setWaiting(false);
    return;
}

            const newState = await stepBattle(battleState, [playerAction, aiAction]);
            setBattleState(newState);
            setLastMoves(prev => ({
                ...prev,
                ai: aiAction.moveId || undefined
            }));
            await finishBattle(newState);
        } catch (err) {
            console.error('Switch error:', err);
            setStatusText('交代に失敗しました。');
        }

        setWaiting(false);
    };

    if (loading) {
        return (
            <div className="flex min-h-dvh items-center justify-center bg-[var(--surface-1)]">
                <div className="text-lg text-[var(--text-muted)]">バトル準備中...</div>
            </div>
        );
    }

    if (!battleState) {
        return (
            <div className="flex min-h-dvh items-center justify-center bg-[var(--surface-1)]">
                <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] px-6 py-5 text-center">
                    <p className="text-lg font-medium text-[var(--text-primary)]">対戦開始を待っています...</p>
                    <p className="mt-2 text-sm text-[var(--text-muted)]">
                        {statusText || 'ホストが初期盤面を準備中です。'}
                    </p>
                </div>
            </div>
        );
    }

    const player = getPlayer(localPlayerId);
const ai = getPlayer(opponentPlayerId);

if (!player || !ai) {
    return (
        <div className="flex min-h-dvh items-center justify-center bg-[var(--surface-1)]">
            <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] px-6 py-5 text-center">
                <p className="text-lg font-medium text-[var(--text-primary)]">対戦情報を同期中です...</p>
                <p className="mt-2 text-sm text-[var(--text-muted)]">
                    プレイヤー情報を取得しています。
                </p>
            </div>
        </div>
    );
}

const playerPokemon = player.team[player.activeSlot];
const aiPokemon = ai.team[ai.activeSlot];

if (!playerPokemon || !aiPokemon) {
    return (
        <div className="flex min-h-dvh items-center justify-center bg-[var(--surface-1)]">
            <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] px-6 py-5 text-center">
                <p className="text-lg font-medium text-[var(--text-primary)]">対戦情報を同期中です...</p>
                <p className="mt-2 text-sm text-[var(--text-muted)]">
                    ポケモン情報を取得しています。
                </p>
            </div>
        </div>
    );
}

const playerSpecies = species[playerPokemon.speciesId];
const aiSpecies = species[aiPokemon.speciesId];

if (!playerSpecies || !aiSpecies) {
    return (
        <div className="flex min-h-dvh items-center justify-center bg-[var(--surface-1)]">
            <div className="rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] px-6 py-5 text-center">
                <p className="text-lg font-medium text-[var(--text-primary)]">対戦情報を同期中です...</p>
                <p className="mt-2 text-sm text-[var(--text-muted)]">
                    種族データを取得しています。
                </p>
            </div>
        </div>
    );
}

const mustSwitch = needsForcedSwitch(battleState, localPlayerId);

const playerLastMove = lastMoves.player ? moves[lastMoves.player] : undefined;
const aiLastMove = lastMoves.ai ? moves[lastMoves.ai] : undefined;

    return (
        <div className="flex min-h-dvh flex-col bg-[var(--surface-1)]">
            <header className="border-b border-[var(--border)] bg-[var(--surface-2)]">
                <div className="mx-auto flex max-w-4xl items-center justify-between px-4 py-3">
                    <div className="flex items-center gap-3">
                        <button
                            onClick={() => {
                                if (battleMode === 'player') {
                                    clearOnlineSession();
                                }
                                navigate('/home');
                            }}
                            className="rounded-lg p-2 transition-colors hover:bg-[var(--surface-3)]"
                            aria-label="ホームに戻る"
                        >
                            <ArrowLeft className="size-5 text-[var(--text-muted)]" />
                        </button>
                        <span className="font-medium tabular-nums text-[var(--text-primary)]">ターン {battleState.turn}</span>
                    </div>
                    <span className="text-sm text-[var(--text-muted)]">
                        {battleMode === 'player' ? 'VS Player (PeerJS)' : 'VS AI (Minimax)'}
                    </span>
                </div>
                {statusText && (
                    <div className="border-t border-[var(--border)] px-4 py-2 text-center text-sm text-[var(--text-muted)]">
                        {statusText}
                    </div>
                )}
            </header>

            <main className="mx-auto grid h-[calc(100dvh-65px)] w-full max-w-7xl grid-cols-1 gap-4 overflow-hidden px-4 py-4 lg:grid-cols-[minmax(0,1fr)_420px]">
            <section className="flex min-h-0 flex-col gap-3 overflow-hidden">
                <div className="flex items-start gap-4">
                    <TeamIndicator team={ai.team} activeSlot={ai.activeSlot} species={species} isPlayer={false} />
                    <PokemonStatus
                        creature={aiPokemon}
                        species={aiSpecies}
                        isPlayer={false}
                    />
                </div>
                
                <BattleFieldStatusPanel
                    field={(battleState as BattleStateWithField).field}
                    localPlayerId={localPlayerId}
                    opponentPlayerId={opponentPlayerId}
                />

                <ActionSummary
                    playerMove={playerLastMove ? { name: playerLastMove.name, type: playerLastMove.type } : undefined}
                    aiMove={aiLastMove ? { name: aiLastMove.name, type: aiLastMove.type } : undefined}
                    getTypeColor={getTypeColor}
                />

                <div className="flex items-end gap-4">
                    <TeamIndicator team={player.team} activeSlot={player.activeSlot} species={species} isPlayer={true} />
                    <PokemonStatus
                        creature={playerPokemon}
                        species={playerSpecies}
                        isPlayer={true}
                    />
                </div>

                <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-4">
                <div className="mb-3 grid grid-cols-2 gap-2">
    <button
        onClick={() => setCommandMode('fight')}
        className={cn(
            'rounded-lg p-2 text-sm font-medium transition-all',
            commandMode === 'fight'
                ? 'bg-[var(--accent)] text-white'
                : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]'
        )}
    >
        たたかう
    </button>

    <button
        onClick={() => setCommandMode('pokemon')}
        className={cn(
            'rounded-lg p-2 text-sm font-medium transition-all',
            commandMode === 'pokemon'
                ? 'bg-[var(--accent)] text-white'
                : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]'
        )}
    >
        ニキモン
    </button>
</div>
                    {false ? null : (
                        <div>
                            <div className="mb-3 grid grid-cols-2 gap-2">
                                {playerPokemon.moves.map((moveId) => {
                                    const move = moves[moveId];
                                    const pp = playerPokemon.movePp[moveId] ?? 10;

                                    if (!move) return null;

                                    const categoryLabel =
                                    move.category === 'physical'
                                        ? '物理'
                                        : move.category === 'special'
                                            ? '特殊'
                                            : move.category === 'status'
                                                ? '変化'
                                                : move.category ?? '-';
                                
                                const accuracyLabel =
                                    typeof move.accuracy === 'number'
                                        ? Math.round(move.accuracy * 100)
                                        : '-';
                                
                                const moveDescription = move.description || '説明なし';

                                const targetTypes = aiPokemon.types ?? species[aiPokemon.speciesId]?.type ?? [];

const shouldShowEffectiveness =
    move.category !== 'status' &&
    typeof move.power === 'number' &&
    move.power > 0;

const effectiveness = shouldShowEffectiveness
    ? getTypeEffectiveness(move.type, targetTypes)
    : null;

const effectivenessLabel = getEffectivenessLabel(effectiveness);

                                    return (
                                        <div key={moveId} className="group relative">
                                            <button
                                                onClick={() => handleSelectMove(moveId)}
                                                disabled={waiting || pp === 0}
                                                className={cn(
                                                    'w-full rounded-xl border p-3 text-left transition-all',
                                                    waiting || pp === 0
                                                        ? 'cursor-not-allowed border-[var(--border)] bg-[var(--surface-3)] opacity-50'
                                                        : 'border-[var(--border)] bg-[var(--surface-3)] hover:border-[var(--border-hover)] hover:bg-[var(--surface-4)]',
                                                )}
                                            >
                                                <div className="mb-2 flex items-center justify-between gap-2">
    <span className="min-w-0 flex-1 truncate font-medium text-[var(--text-primary)]">
        {move.name}
    </span>

    <div className="flex shrink-0 items-center gap-1">
        {effectivenessLabel && (
            <span
                className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${getEffectivenessClass(effectiveness)}`}
            >
                {effectivenessLabel}
                {formatEffectivenessMultiplier(effectiveness)}
            </span>
        )}

<span
    className="rounded-full px-2 py-0.5 text-xs text-white"
    style={{ backgroundColor: getTypeColor(move.type) }}
>
    {getTypeLabel(move.type)}
</span>
    </div>
</div>
                                    
                                                <div className="flex items-center justify-between text-xs text-[var(--text-muted)]">
                                                    <span>{categoryLabel}</span>
                                                    <span>
                                                    威力 {move.power ?? '-'} / 命中 {accuracyLabel}
                                                    </span>
                                                </div>
                                    
                                                <div className="mt-1 text-xs tabular-nums text-[var(--text-muted)]">
                                                    PP: {pp}
                                                </div>
                                            </button>
                                    
                                            <div className="pointer-events-none absolute bottom-full left-0 z-30 mb-2 hidden w-80 rounded-2xl border border-[var(--border)] bg-[var(--surface-2)] p-4 shadow-2xl group-hover:block group-focus-within:block">
                                                <div className="mb-3 flex items-start justify-between gap-3">
                                                    <div>
                                                        <div className="text-sm font-bold text-[var(--text-primary)]">
                                                            {move.name}
                                                        </div>
                                                        <div className="mt-1 text-xs text-[var(--text-muted)]">
                                                            {categoryLabel}
                                                        </div>
                                                    </div>
                                    
                                                    <div className="flex shrink-0 flex-col items-end gap-1">
                                                    <span
    className="rounded-full px-2 py-1 text-xs text-white"
    style={{ backgroundColor: getTypeColor(move.type) }}
>
    {getTypeLabel(move.type)}
</span>
    {effectivenessLabel && (
        <span
            className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${getEffectivenessClass(effectiveness)}`}
        >
            {effectivenessLabel}
            {formatEffectivenessMultiplier(effectiveness)}
        </span>
    )}
</div>
                                                </div>
                                    
                                                <div className="mb-3 grid grid-cols-3 gap-2 text-xs">
                                                    <div className="rounded-lg bg-[var(--surface-3)] px-3 py-2">
                                                        <div className="text-[10px] text-[var(--text-muted)]">威力</div>
                                                        <div className="font-semibold text-[var(--text-primary)]">
                                                            {move.power ?? '-'}
                                                        </div>
                                                    </div>
                                                    <div className="rounded-lg bg-[var(--surface-3)] px-3 py-2">
                                                        <div className="text-[10px] text-[var(--text-muted)]">命中</div>
                                                        <div className="font-semibold text-[var(--text-primary)]">
                                                        {accuracyLabel}
                                                        </div>
                                                    </div>
                                                    <div className="rounded-lg bg-[var(--surface-3)] px-3 py-2">
                                                        <div className="text-[10px] text-[var(--text-muted)]">PP</div>
                                                        <div className="font-semibold text-[var(--text-primary)]">
                                                            {pp}
                                                        </div>
                                                    </div>
                                                </div>
                                    
                                                <div className="rounded-lg bg-[var(--surface-3)] px-3 py-3 text-xs leading-relaxed text-[var(--text-primary)]">
                                                    {moveDescription}
                                                </div>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        </div>
                    )}
                </div>
                </section>

                <aside className="min-h-0 rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-3">
            <div ref={logsRef} className="h-full overflow-y-auto pr-1">
        <BattleLog
            logs={battleState.log}
            currentTurn={battleState.turn}
        />
            </div>
        </aside>
        {(commandMode === 'pokemon' || mustSwitch) && (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 px-4">
        <div className="grid h-[80dvh] w-full max-w-7xl grid-cols-[320px_minmax(0,1fr)_320px] gap-5">
            {/* Left column: player team list */}
            <div className="flex min-h-0 flex-col space-y-2 rounded-xl bg-[var(--surface-2)] p-4">
                <div className="mb-2 text-sm font-bold text-[var(--text-primary)]">
                    味方チーム
                </div>

                {player.team.map((mon, idx) => {
                    const monSpecies = species[mon.speciesId];
                    const isActive = idx === player.activeSlot;
                    const isFainted = mon.hp <= 0;

                    return (
                        <button
                            key={idx}
                            onClick={() => setFocusedTeamSlot(idx)}
                            disabled={isFainted}
                            className={cn(
                                'w-full rounded-lg border p-2 text-left transition-all',
                                focusedTeamSlot === idx
                                    ? 'border-[var(--accent)] bg-[var(--accent-muted)]'
                                    : isActive
                                        ? 'border-[var(--accent)]/50 bg-[var(--surface-3)]'
                                        : 'border-[var(--border)] bg-[var(--surface-3)] hover:border-[var(--border-hover)]',
                                isFainted && 'opacity-50'
                            )}
                        >
                            <div className="truncate text-sm font-medium text-[var(--text-primary)]">
                                {monSpecies?.name}
                            </div>
                            <div className="text-xs tabular-nums text-[var(--text-muted)]">
                                HP {mon.hp}/{mon.maxHp}
                            </div>
                        </button>
                    );
                })}
            </div>

            {/* Center column: focused pokemon detail */}
            <div className="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] rounded-xl bg-[var(--surface-2)] p-6">
                {(() => {
                    const mon = player.team[focusedTeamSlot] ?? player.team[player.activeSlot];
                    const monSpecies = species[mon.speciesId];
                    const isActive = focusedTeamSlot === player.activeSlot;
                    const isFainted = mon.hp <= 0;
                    const statusLabel = mon.statuses.length > 0
                        ? mon.statuses.map((status) => getStatusLabel(status.id)).join(' / ')
                        : 'なし';

                    if (!monSpecies) return null;

                    return (
                        <>
                            {/* 上段 */}
                            <div className="grid grid-cols-[minmax(0,1fr)_360px] gap-4">
                                {/* 左：基本情報 */}
                                <div className="space-y-4">
                                    <div className="mb-3 flex items-start justify-between gap-3">
                                        <div>
                                            <div className="text-xl font-bold text-[var(--text-primary)]">
                                                {monSpecies?.name}
                                            </div>
                                            <div className="mt-1 flex gap-1">
                                                {(monSpecies?.type ?? []).map((t) => (
                                                    <span
                                                        key={t}
                                                        className="rounded-full px-2 py-0.5 text-xs text-white"
                                                        style={{ backgroundColor: getTypeColor(t) }}
                                                    >
                                                        {getTypeLabel(t)}
                                                    </span>
                                                ))}
                                            </div>
                                            <div className="mt-1 text-sm text-[var(--text-muted)]">
                                                HP {mon.hp}/{mon.maxHp}
                                            </div>
                                            <div className="mt-2 h-2 w-full rounded-full bg-[var(--surface-3)]">
                                                <div
                                                    className="h-full rounded-full bg-emerald-500"
                                                    style={{
                                                        width: `${(mon.hp / mon.maxHp) * 100}%`,
                                                    }}
                                                />
                                            </div>
                                            <div className="mt-4 rounded-lg bg-[var(--surface-3)] p-3">
                                                <div className="text-xs text-[var(--text-muted)]">特性</div>
                                                <div className="font-bold text-[var(--text-primary)]">
                                                    {mon.ability ? getAbilityLabel(mon.ability) : 'なし'}
                                                </div>
                                            </div>
                                        </div>
                                        <button
                                            onClick={() => setCommandMode('fight')}
                                            disabled={mustSwitch}
                                            className="rounded-lg bg-[var(--surface-3)] px-3 py-1 text-sm text-[var(--text-muted)] disabled:opacity-40"
                                        >
                                            戻る
                                        </button>
                                    </div>
                                </div>

                                {/* 右：種族値 */}
                                <div className="rounded-xl bg-[var(--surface-3)] p-4">
                                    <div className="mb-3 text-sm font-semibold text-[var(--text-muted)]">
                                        種族値
                                    </div>
                                    {(() => {
                                        const stats = monSpecies?.baseStats;
                                        if (!stats) return null;
                                        const evs = {
                                            hp: 0,
                                            atk: 252,
                                            def: 0,
                                            spa: 252,
                                            spd: 4,
                                            spe: 0,
                                        };
                                        const total = stats.hp + stats.atk + stats.def + stats.spa + stats.spd + stats.spe;
                                        const renderBar = (label: string, value: number, ev: number, max: number) => {
                                            const percentage = Math.min(100, (value / max) * 100);
                                            const evBonus = Math.floor(ev / 4);
                                            const evPercentage = Math.min(100, (evBonus / max) * 100);
                                            return (
                                                <div className="grid grid-cols-[64px_1fr_40px] items-center gap-2 text-xs">
                                                    <span className="text-[var(--text-muted)]">{label}</span>
                                                    <div className="relative h-2.5 overflow-hidden rounded-full bg-[var(--surface-4)]">
                                                        <div
                                                            className="absolute left-0 top-0 h-full rounded-full bg-amber-400"
                                                            style={{ width: `${Math.min(100, percentage + evPercentage)}%` }}
                                                        />
                                                        <div
                                                            className="absolute left-0 top-0 h-full rounded-full bg-[var(--accent)]"
                                                            style={{ width: `${percentage}%` }}
                                                        />
                                                    </div>
                                                    <span className="text-right tabular-nums text-[var(--text-primary)]">
                                                        {value}
                                                    </span>
                                                </div>
                                            );
                                        };
                                        return (
                                            <div className="space-y-2">
                                                {renderBar('HP', stats.hp, evs.hp, 255)}
                                                {renderBar('攻撃', stats.atk, evs.atk, 255)}
                                                {renderBar('防御', stats.def, evs.def, 255)}
                                                {renderBar('特攻', stats.spa, evs.spa, 255)}
                                                {renderBar('特防', stats.spd, evs.spd, 255)}
                                                {renderBar('素早さ', stats.spe, evs.spe, 255)}
                                                {renderBar('合計', total, 0, 720)}
                                            </div>
                                        );
                                    })()}
                                </div>
                            </div>

                            {/* 下段 */}
                            <div className="mt-4 space-y-3">
                                <div className="rounded-lg bg-[var(--surface-3)] p-3">
                                    <div className="text-xs text-[var(--text-muted)]">状態</div>
                                    <div className="font-bold text-[var(--text-primary)]">{statusLabel}</div>
                                </div>
                                <div className="grid grid-cols-2 gap-2">
                                    {mon.moves?.map((moveId) => {
                                        const move = moves[moveId];
                                        if (!move) return null;
                                        return (
                                            <div
                                                key={moveId}
                                                className="rounded-lg border border-[var(--border)] bg-[var(--surface-3)] px-3 py-3 text-sm"
                                            >
                                                <div className="truncate font-medium text-[var(--text-primary)]">
                                                    {move.name}
                                                </div>
                                                <div className="mt-1 flex items-center justify-between text-xs text-[var(--text-muted)]">
                                                    <span>{move.power ?? '-'}</span>
                                                    <span
                                                        className="rounded-full px-2 text-white"
                                                        style={{ backgroundColor: getTypeColor(move.type) }}
                                                    >
                                                        {getTypeLabel(move.type)}
                                                    </span>
                                                </div>
                                            </div>
                                        );
                                    })}
                                </div>
                                <button
                                    onClick={() => handleSwitch(focusedTeamSlot)}
                                    disabled={isActive || isFainted || waiting}
                                    className="w-full rounded-xl bg-[var(--accent)] p-3 font-bold text-white disabled:opacity-40"
                                >
                                    {isActive ? '場に出ています' : isFainted ? 'ひんしです' : '交代する'}
                                </button>
                            </div>
                        </>
                    );
                })()}
            </div>

            {/* Right column: opponent team list */}
            <div className="flex min-h-0 flex-col space-y-2 rounded-xl bg-[var(--surface-2)] p-4">
                <div className="mb-2 text-sm font-bold text-[var(--text-primary)]">
                    相手チーム
                </div>

                {ai.team.map((mon, idx) => {
                    const monSpecies = species[mon.speciesId];
                    const isActive = idx === ai.activeSlot;
                    const isFainted = mon.hp <= 0;

                    return (
                        <div
                            key={idx}
                            className={cn(
                                'rounded-lg border p-2 text-left transition-all',
                                isActive
                                    ? 'border-[var(--accent)] bg-[var(--accent-muted)]'
                                    : 'border-[var(--border)] bg-[var(--surface-3)]',
                                isFainted && 'opacity-50'
                            )}
                        >
                            <div className="truncate text-sm font-medium text-[var(--text-primary)]">
                                {monSpecies?.name ?? '???'}
                            </div>
                        </div>
                    );
                })}
            </div>
        </div>
    </div>
)}
        </main>
        </div>
    );
}

// Team indicator showing remaining pokemon HP
function TeamIndicator({
    team,
    activeSlot,
    species,
    isPlayer
}: {
    team: CreatureStateWire[];
    activeSlot: number;
    species: SpeciesData;
    isPlayer: boolean;
}) {
    return (
        <div className={cn(
            'flex flex-col gap-1',
            isPlayer ? 'items-end' : 'items-start'
        )}>
            {team.map((mon, idx) => {
                const hpPercent = mon.maxHp > 0 ? (mon.hp / mon.maxHp) * 100 : 0;
                const isActive = idx === activeSlot;
                const isFainted = mon.hp <= 0;
                const monSpecies = species[mon.speciesId];

                return (
                    <div
                        key={idx}
                        className={cn(
                            'flex items-center gap-2 rounded-full px-2 py-1 text-xs',
                            isActive ? 'bg-[var(--accent-muted)]' : 'bg-[var(--surface-3)]'
                        )}
                        title={`${monSpecies?.name}: ${mon.hp}/${mon.maxHp} HP`}
                    >
                        <span className={cn(
                            'size-2 rounded-full',
                            isFainted ? 'bg-red-500' : isActive ? 'bg-[var(--accent)]' : 'bg-[var(--text-muted)]'
                        )} />
                        <div className="h-1.5 w-12 overflow-hidden rounded-full bg-[var(--surface-4)]">
                            <div
                                className={cn(
                                    'h-full transition-all',
                                    hpPercent > 50 ? 'bg-emerald-500' : hpPercent > 20 ? 'bg-amber-500' : 'bg-red-500'
                                )}
                                style={{ width: `${hpPercent}%` }}
                            />
                        </div>
                    </div>
                );
            })}
        </div>
    );
}

function PokemonStatus({
    creature,
    species,
    isPlayer
}: {
    creature: CreatureStateWire;
    species: SpeciesData[string] | undefined;
    isPlayer: boolean;
}) {
    const hpPercentage = creature.maxHp > 0 ? (creature.hp / creature.maxHp) * 100 : 0;
    const hpColor = hpPercentage > 50 ? 'bg-emerald-500' : hpPercentage > 20 ? 'bg-amber-500' : 'bg-red-500';

    return (
        <div className={cn('flex-1', isPlayer ? 'text-right' : 'text-left')}>
            <div className="inline-block min-w-64 rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-4">
                <div className={cn('flex items-center gap-3', isPlayer ? 'flex-row-reverse' : '')}>
                    <div className="text-3xl">
                        {isPlayer ? '🔵' : '🔴'}
                    </div>
                    <div className={isPlayer ? 'text-right' : ''}>
                        <h3 className="text-balance text-lg font-bold text-[var(--text-primary)]">{species?.name || creature.name}</h3>
                        <div className={cn('flex gap-1', isPlayer ? 'justify-end' : '')}>
                        {(creature.types || species?.type || []).map((t) => (
    <span
        key={t}
        className="rounded-full px-2 py-0.5 text-xs text-white"
        style={{ backgroundColor: getTypeColor(t) }}
    >
        {getTypeLabel(t)}
    </span>
))}
                        </div>
                    </div>
                </div>

                {/* HP Bar */}
                <div className="mt-3">
                    <div className="mb-1 flex justify-between text-xs text-[var(--text-muted)]">
                        <span>HP</span>
                        <span className="tabular-nums">{creature.hp}/{creature.maxHp}</span>
                    </div>
                    <div className="h-2.5 overflow-hidden rounded-full bg-[var(--surface-4)]">
                        <div
                            className={cn('h-full transition-all duration-300', hpColor)}
                            style={{ width: `${hpPercentage}%` }}
                        />
                    </div>
                </div>

                {/* Stat Stages */}
                {(() => {
                    const stages = creature.stages;
                    const displayStages: { label: string; value: number }[] = [];
                    if (stages.atk !== 0) displayStages.push({ label: 'こうげき', value: stages.atk });
                    if (stages.def !== 0) displayStages.push({ label: 'ぼうぎょ', value: stages.def });
                    if (stages.spa !== 0) displayStages.push({ label: 'とくこう', value: stages.spa });
                    if (stages.spd !== 0) displayStages.push({ label: 'とくぼう', value: stages.spd });
                    if (stages.spe !== 0) displayStages.push({ label: 'すばやさ', value: stages.spe });

                    return displayStages.length > 0 && (
                        <div className="mt-2 flex flex-wrap gap-1">
                            {displayStages.map(({ label, value }) => (
                                <span
                                    key={label}
                                    className={cn(
                                        'rounded px-2 py-0.5 text-xs font-medium tabular-nums text-white',
                                        value > 0 ? 'bg-green-600' : 'bg-red-600'
                                    )}
                                >
                                    {value > 0 ? '+' : ''}{value} {label}
                                </span>
                            ))}
                        </div>
                    );
                })()}

                {/* Status */}
                {creature.statuses && creature.statuses.length > 0 && (
                    <div className="mt-2 flex flex-wrap gap-1">
                        {creature.statuses.map((status, i) => (
                            <span key={i} className="rounded bg-purple-600 px-2 py-0.5 text-xs text-white">
                                {getStatusLabel(status.id)}
                            </span>
                        ))}
                    </div>
                )}
            </div>
        </div>
    );
}
