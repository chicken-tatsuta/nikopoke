import { useState, useEffect, useRef, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { ArrowLeft } from 'lucide-react';
import { cn } from '../lib/cn';
import { loadAllData, getTypeColor } from '../lib/data';
import { BattleLog } from '../components/BattleLog';
import { getAbilityLabel } from './PokemonDetailPage';

import { uploadGlobalBattleRecord, createBattleStatsId } from '../lib/globalBattleStats';
import { useAuth } from '../contexts/AuthContext';

import {
    initEngine,
    createBattleState,
    stepBattle,
    getFirstAvailableSwitchSlot,
    getBestMoveMinimax,
    getBestMoveVega,
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
    getPrecomputedAiAction,
    makeAiStateKey,
    precomputeAiAction,
} from '../lib/aiPrecompute';
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
    global?: Record<string, FieldEffectValue> | Array<{ id: string; remainingTurns?: number | null; remaining?: number | null; turns?: number | null; layers?: number }>;
    sides?:
        | Array<Record<string, FieldEffectValue> | Array<{ id: string; remainingTurns?: number | null; remaining?: number | null; turns?: number | null; layers?: number }>>
        | Record<string, Record<string, FieldEffectValue> | Array<{ id: string; remainingTurns?: number | null; remaining?: number | null; turns?: number | null; layers?: number }>>;
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

const WEATHER_FIELD_IDS = new Set(['sun', 'rain', 'sandstorm', 'snow']);

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

function normalizeEffects(
    effects?: Record<string, FieldEffectValue> | Array<{ id: string; remainingTurns?: number | null; remaining?: number | null; turns?: number | null; layers?: number }>,
): FieldEffectItem[] {
    if (!effects) return [];

    if (Array.isArray(effects)) {
        return effects
            .filter((effect) => effect.id && (effect.remainingTurns == null || effect.remainingTurns > 0))
            .map((effect) => ({
                key: effect.id,
                label: getEffectLabel(effect.id),
                turns:
                    typeof effect.remainingTurns === 'number'
                        ? effect.remainingTurns
                        : typeof effect.remaining === 'number'
                            ? effect.remaining
                            : typeof effect.turns === 'number'
                                ? effect.turns
                                : undefined,
                layers: typeof effect.layers === 'number' ? effect.layers : undefined,
            }));
    }

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

function getBattleWeatherId(field?: BattleFieldLike): string | null {
    return normalizeEffects(field?.global).find((effect) => WEATHER_FIELD_IDS.has(effect.key))?.key ?? null;
}

function getBattleWeatherClass(weatherId: string | null): string {
    switch (weatherId) {
        case 'sun':
            return 'battle-weather-sun';
        case 'rain':
            return 'battle-weather-rain';
        case 'sandstorm':
            return 'battle-weather-sandstorm';
        case 'snow':
            return 'battle-weather-snow';
        default:
            return 'battle-weather-none';
    }
}

function getSideField(
    sides: BattleFieldLike['sides'],
    playerId: string,
    fallbackIndex: number,
): Record<string, FieldEffectValue> | Array<{ id: string; remainingTurns?: number | null; remaining?: number | null; turns?: number | null; layers?: number }> | undefined {
    if (!sides) return undefined;

    if (Array.isArray(sides)) {
        return sides[fallbackIndex];
    }

    return sides[playerId];
}

function formatFieldEffects(label: string, effects: FieldEffectItem[]): string | null {
    if (effects.length === 0) {
        return null;
    }

    const details = effects
        .map((effect) => {
            const suffix = [
                effect.layers && effect.layers > 1 ? `${effect.layers}層` : null,
                effect.turns ? `あと${effect.turns}T` : null,
            ]
                .filter(Boolean)
                .join('/');

            return suffix ? `${effect.label}(${suffix})` : effect.label;
        })
        .join('、');

    return `${label}: ${details}`;
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
    const fieldText = [
        formatFieldEffects('場', globalEffects),
        formatFieldEffects('相手側', opponentSideEffects),
        formatFieldEffects('自分側', playerSideEffects),
    ].filter(Boolean).join(' / ');

    return (
        <div className="flex justify-end">
            <div className="max-w-full truncate text-right text-xs text-[var(--text-muted)]">
                {fieldText ? `場の状態 / ${fieldText}` : '場の状態 / なし'}
            </div>
        </div>
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

type BattlePlaybackState = {
    isPlaying: boolean;
    label: string;
    attackingPlayerId?: string;
    damagedPlayerId?: string;
    effectType?: string;
    statusFlashPlayerId?: string;
    statusFlashType?: BattleStatusFlashType;
    faintedCreatureIds: string[];
};

type BattleStatusFlashType = 'poison' | 'burn' | 'paralysis' | 'sleep' | 'freeze' | 'confusion';

type BattlePopup = {
    id: number;
    tone: 'ability' | 'info';
    side: 'player' | 'opponent' | 'center';
    title: string;
    text: string;
};

const IDLE_PLAYBACK_STATE: BattlePlaybackState = {
    isPlaying: false,
    label: '行動選択中',
    faintedCreatureIds: [],
};

const PLAYBACK_STEP_MS = 800;
const PLAYBACK_HIT_MS = 1300;
const PLAYBACK_FAINT_MS = 2000;
const VEGA_DEPTH = 3;
const VEGA_PRECOMPUTE_BRANCH_LIMIT = 3;
const VEGA_PRECOMPUTE_MAX_WAIT_MS = 3000;
const BATTLE_POPUP_MS = 1400;
const HIDDEN_BATTLE_STATUS_IDS = new Set(['pending_switch']);
const STATUS_FLASH_COLORS: Record<BattleStatusFlashType, string> = {
    poison: '#a855f7',
    burn: '#f97316',
    paralysis: '#facc15',
    sleep: '#818cf8',
    freeze: '#38bdf8',
    confusion: '#ec4899',
};
const POKEMON_IMAGE_MODULES = import.meta.glob('../../image/*.{png,jpg,jpeg,webp,avif}', {
    eager: true,
    query: '?url',
    import: 'default',
}) as Record<string, string>;
const POKEMON_IMAGE_BY_ID = Object.fromEntries(
    Object.entries(POKEMON_IMAGE_MODULES).map(([path, url]) => {
        const filename = path.split('/').pop() ?? '';
        const id = filename.replace(/\.(png|jpe?g|webp|avif)$/i, '').toLowerCase();
        return [id, url];
    }),
);

function wait(ms: number): Promise<void> {
    return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function cloneBattleState(state: BattleStateWire): BattleStateWire {
    return structuredClone(state) as BattleStateWire;
}

function findPlayerInState(state: BattleStateWire, playerId: string): PlayerStateWire | undefined {
    return state.players.find((player) => player.id === playerId);
}

function findActiveCreature(state: BattleStateWire, playerId: string): CreatureStateWire | undefined {
    const player = findPlayerInState(state, playerId);
    return player?.team[player.activeSlot];
}

function findStatusFlashTargetFromName(
    state: BattleStateWire,
    log: string,
    statusType: BattleStatusFlashType,
): { playerId: string; statusType: BattleStatusFlashType } | null {
    const matchedCreature = state.players
        .flatMap((player) => player.team.map((creature) => ({ playerId: player.id, name: creature.name })))
        .find(({ name }) => log.includes(name));
    return matchedCreature ? { playerId: matchedCreature.playerId, statusType } : null;
}

function getStatusFlashFromLogs(
    logs: string[],
    state: BattleStateWire,
    fallbackPlayerId?: string,
): { playerId: string; statusType: BattleStatusFlashType } | null {
    for (const log of logs) {
        if (log.includes('効かない') || log.includes('効かなかった') || log.includes('すでに')) {
            continue;
        }
        if (log.includes('どくの ダメージ') || log.includes('もうどくの ダメージ') || log.includes('毒をあびた')) {
            const target = findStatusFlashTargetFromName(state, log, 'poison');
            if (target) return target;
        }
        if (log.includes('やけどのダメージ')) {
            const target = findStatusFlashTargetFromName(state, log, 'burn');
            if (target) return target;
        }
        if (log.includes('しびれて 動けない')) {
            return fallbackPlayerId ? { playerId: fallbackPlayerId, statusType: 'paralysis' } : null;
        }
        if (log.includes('眠り続けている') || log.includes('眠りながら')) {
            const target = findStatusFlashTargetFromName(state, log, 'sleep');
            if (target) return target;
        }
        if (log.includes('凍りついて 動けない') || log.includes('こおりが とけた')) {
            const target = findStatusFlashTargetFromName(state, log, 'freeze');
            if (target) return target;
        }
        if (log.includes('わけもわからず 自分を 攻撃した')) {
            return fallbackPlayerId ? { playerId: fallbackPlayerId, statusType: 'confusion' } : null;
        }
    }
    return null;
}

function isStatusNoEffectLog(log: string): boolean {
    return (
        log.includes('効かない') ||
        log.includes('効かなかった') ||
        log.includes('すでに')
    );
}

function copyFinalActiveCreature(
    draft: BattleStateWire,
    finalState: BattleStateWire,
    playerId: string,
) {
    const draftPlayer = findPlayerInState(draft, playerId);
    const finalPlayer = findPlayerInState(finalState, playerId);
    if (!draftPlayer || !finalPlayer) {
        return;
    }

    draftPlayer.activeSlot = finalPlayer.activeSlot;
    draftPlayer.team[finalPlayer.activeSlot] = structuredClone(finalPlayer.team[finalPlayer.activeSlot]) as CreatureStateWire;
}

function copyFinalActiveCreatureBeforeLoggedHpDelta(
    draft: BattleStateWire,
    finalState: BattleStateWire,
    playerId: string,
    logs: string[],
): number | null {
    copyFinalActiveCreature(draft, finalState, playerId);

    const pair = activeCreaturePair(draft, finalState, playerId);
    if (!pair) {
        return null;
    }

    const loggedDelta = getLoggedHpDelta(logs, pair.draftCreature.name);
    if (loggedDelta === null) {
        return null;
    }

    pair.draftCreature.hp = Math.max(
        0,
        Math.min(pair.draftCreature.maxHp, pair.finalCreature.hp - loggedDelta),
    );
    return loggedDelta;
}

function activeCreaturePair(
    draft: BattleStateWire,
    finalState: BattleStateWire,
    playerId: string,
): { draftCreature: CreatureStateWire; finalCreature: CreatureStateWire } | null {
    const draftPlayer = findPlayerInState(draft, playerId);
    const finalPlayer = findPlayerInState(finalState, playerId);
    if (!draftPlayer || !finalPlayer) {
        return null;
    }

    const draftCreature = draftPlayer.team[draftPlayer.activeSlot];
    const finalCreature = finalPlayer.team[finalPlayer.activeSlot];
    if (!draftCreature || !finalCreature) {
        return null;
    }

    return { draftCreature, finalCreature };
}

function getLoggedHpDelta(logs: string[], creatureName: string): number | null {
    let delta = 0;
    let found = false;
    const escapedName = creatureName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const damagePattern = new RegExp(`${escapedName}は\\s*(\\d+)ダメージ`);
    const healPattern = new RegExp(`${escapedName}.*?(\\d+).*?回復`);

    for (const log of logs) {
        const damage = log.match(damagePattern);
        if (damage) {
            delta -= Number(damage[1]);
            found = true;
            continue;
        }

        const heal = log.match(healPattern);
        if (heal) {
            delta += Number(heal[1]);
            found = true;
        }
    }

    return found ? delta : null;
}

function copyTargetAfterAction(
    draft: BattleStateWire,
    finalState: BattleStateWire,
    playerId: string,
    actionLogs: string[],
    hasLaterAction: boolean,
) {
    const before = activeCreaturePair(draft, finalState, playerId);
    if (!before) {
        return;
    }

    const previousHp = before.draftCreature.hp;
    const loggedDelta = getLoggedHpDelta(actionLogs, before.draftCreature.name);
    copyFinalActiveCreature(draft, finalState, playerId);

    if (!hasLaterAction) {
        return;
    }

    const after = activeCreaturePair(draft, finalState, playerId);
    if (!after) {
        return;
    }

    if (loggedDelta !== null) {
        after.draftCreature.hp = Math.max(0, Math.min(after.draftCreature.maxHp, previousHp + loggedDelta));
        return;
    }

    after.draftCreature.hp = after.finalCreature.hp < previousHp ? after.finalCreature.hp : previousHp;
}

function copyCurrentSlotAfterAction(
    draft: BattleStateWire,
    finalState: BattleStateWire,
    playerId: string,
    actionLogs: string[],
) {
    const draftPlayer = findPlayerInState(draft, playerId);
    const finalPlayer = findPlayerInState(finalState, playerId);
    if (!draftPlayer || !finalPlayer) {
        return;
    }

    const slot = draftPlayer.activeSlot;
    const draftCreature = draftPlayer.team[slot];
    const finalCreature = finalPlayer.team[slot];
    if (!draftCreature || !finalCreature) {
        return;
    }

    const previousHp = draftCreature.hp;
    const loggedDelta = getLoggedHpDelta(actionLogs, draftCreature.name);
    draftPlayer.team[slot] = structuredClone(finalCreature) as CreatureStateWire;

    if (loggedDelta !== null) {
        draftPlayer.team[slot].hp = Math.max(0, Math.min(draftCreature.maxHp, previousHp + loggedDelta));
    }
}

function copyActorAfterOwnAction(
    draft: BattleStateWire,
    finalState: BattleStateWire,
    playerId: string,
    actionLogs: string[],
) {
    const pair = activeCreaturePair(draft, finalState, playerId);
    if (!pair) {
        return;
    }

    const { draftCreature, finalCreature } = pair;
    draftCreature.movePp = { ...finalCreature.movePp };

    const loggedDelta = getLoggedHpDelta(actionLogs, draftCreature.name);
    if (loggedDelta !== null && loggedDelta > 0) {
        draftCreature.hp = Math.max(0, Math.min(draftCreature.maxHp, draftCreature.hp + loggedDelta));
    } else if (finalCreature.hp > draftCreature.hp) {
        draftCreature.hp = finalCreature.hp;
    }

    const stageKeys = Object.keys(draftCreature.stages) as Array<keyof CreatureStateWire['stages']>;
    const hasSelfBuff = stageKeys.some((key) => finalCreature.stages[key] > draftCreature.stages[key]);
    if (hasSelfBuff) {
        draftCreature.stages = { ...finalCreature.stages };
    }

    const addedSelfStatus = finalCreature.statuses.length > draftCreature.statuses.length;
    if (addedSelfStatus && actionLogs.some((log) => log.includes(draftCreature.name))) {
        draftCreature.statuses = structuredClone(finalCreature.statuses) as CreatureStateWire['statuses'];
    }
}

function activeSlotChanged(
    draft: BattleStateWire,
    finalState: BattleStateWire,
    playerId: string,
): boolean {
    const draftPlayer = findPlayerInState(draft, playerId);
    const finalPlayer = findPlayerInState(finalState, playerId);
    return Boolean(draftPlayer && finalPlayer && draftPlayer.activeSlot !== finalPlayer.activeSlot);
}

function moveHasEffect(move: MoveData[string] | undefined, effectType: string): boolean {
    const steps = (move as { steps?: Array<{ type?: string }> } | undefined)?.steps ?? [];
    return steps.some((step) => step.type === effectType);
}

function visibleStatuses(statuses: CreatureStateWire['statuses']): CreatureStateWire['statuses'] {
    return statuses.filter((status) => !HIDDEN_BATTLE_STATUS_IDS.has(status.id));
}

function hasPendingSwitchStatus(creature: CreatureStateWire | undefined): boolean {
    return Boolean(creature?.statuses.some((status) => status.id === 'pending_switch'));
}

function isForcedReplacementAction(action: ActionWire, state: BattleStateWire): boolean {
    if (action.type !== 'switch') {
        return false;
    }
    const actor = findActiveCreature(state, action.playerId);
    return Boolean(actor && (actor.hp <= 0 || hasPendingSwitchStatus(actor)));
}

function logsMentionNegativeEffect(logs: string[], creatureName: string): boolean {
    return logs.some((log) => (
        log.includes(creatureName) &&
        !isStatusNoEffectLog(log) &&
        (
            log.includes('ダメージ') ||
            log.includes('下がった') ||
            log.includes('状態になった') ||
            log.includes('どく') ||
            log.includes('やけど') ||
            log.includes('まひ') ||
            log.includes('こおり') ||
            log.includes('ねむり') ||
            log.includes('こんらん') ||
            log.includes('ひるみ') ||
            log.includes('たおれた')
        )
    ));
}

function getVisualImpactPlayerId(
    action: ActionWire,
    state: BattleStateWire,
    targetId: string,
    actionLogs: string[],
    moves: MoveData,
): string | undefined {
    const target = findActiveCreature(state, targetId);
    const actor = findActiveCreature(state, action.playerId);
    const move = action.moveId ? moves[action.moveId] : undefined;

    if (moveHasEffect(move, 'force_switch')) {
        if (!moveHasEffect(move, 'damage')) {
            return action.playerId;
        }
        return targetId;
    }

    if (target && logsMentionNegativeEffect(actionLogs, target.name)) {
        return targetId;
    }

    if (actor && getLoggedHpDelta(actionLogs, actor.name) !== null) {
        return action.playerId;
    }

    if (move?.category === 'status') {
        return undefined;
    }

    return action.targetId ? targetId : undefined;
}

function finalFaintedCreatureIds(finalState: BattleStateWire): string[] {
    return finalState.players
        .flatMap((player) => player.team)
        .filter((creature) => creature.hp <= 0)
        .map((creature) => creature.id);
}

function getMoveType(moveId: string | undefined, moves: MoveData): string {
    return moveId ? moves[moveId]?.type ?? 'normal' : 'normal';
}

function getActionLabel(
    action: ActionWire,
    state: BattleStateWire,
    moves: MoveData,
    localPlayerId: string,
): string {
    const actor = findActiveCreature(state, action.playerId);
    const side = action.playerId === localPlayerId ? 'あなた' : '相手';

    if (action.type === 'switch') {
        if (isForcedReplacementAction(action, state)) {
            return `${side}が出すポケモンを選んでいます`;
        }
        return `${side}が交代しています`;
    }

    const moveName = action.moveId ? moves[action.moveId]?.name : undefined;
    return moveName && actor ? `${side}: ${actor.name}の${moveName}` : `${side}の行動中`;
}

function getActionLogStartIndex(
    action: ActionWire,
    state: BattleStateWire,
    logs: string[],
    moves: MoveData,
): number {
    const player = findPlayerInState(state, action.playerId);
    if (!player) {
        return Number.POSITIVE_INFINITY;
    }

    if (action.type === 'switch') {
        const index = logs.findIndex((log) => log.startsWith(`${player.name}は `) && log.includes('を 繰り出した！'));
        return index >= 0 ? index : Number.POSITIVE_INFINITY;
    }

    const moveName = action.moveId ? moves[action.moveId]?.name ?? action.moveId : undefined;
    if (!moveName) {
        return Number.POSITIVE_INFINITY;
    }

    const index = logs.findIndex((log) => log === `${player.name}の ${moveName}！`);
    return index >= 0 ? index : Number.POSITIVE_INFINITY;
}

function orderActionsByBattleLogs(
    actions: ActionWire[],
    state: BattleStateWire,
    logs: string[],
    moves: MoveData,
): ActionWire[] {
    return actions
        .map((action, index) => ({
            action,
            index,
            logIndex: getActionLogStartIndex(action, state, logs, moves),
        }))
        .filter(({ logIndex }) => Number.isFinite(logIndex))
        .sort((left, right) => {
            if (left.logIndex !== right.logIndex) {
                return left.logIndex - right.logIndex;
            }
            return left.index - right.index;
        })
        .map(({ action }) => action);
}

function getPopupSideFromName(
    state: BattleStateWire | null,
    name: string,
    localPlayerId: string,
    opponentPlayerId: string,
): BattlePopup['side'] {
    if (!state) {
        return 'center';
    }

    const localPlayer = findPlayerInState(state, localPlayerId);
    const opponentPlayer = findPlayerInState(state, opponentPlayerId);

    if (opponentPlayer?.team.some((creature) => creature.name === name)) {
        return 'opponent';
    }
    if (localPlayer?.team.some((creature) => creature.name === name)) {
        return 'player';
    }

    return 'center';
}

function parseAbilityPopup(
    log: string,
    state: BattleStateWire | null,
    localPlayerId: string,
    opponentPlayerId: string,
): Omit<BattlePopup, 'id'> | null {
    const match = log.match(/^(.+)の 特性『(.+)』！$/);
    if (!match) {
        return null;
    }

    return {
        tone: 'ability',
        side: getPopupSideFromName(state, match[1], localPlayerId, opponentPlayerId),
        title: `特性『${match[2]}』`,
        text: match[1],
    };
}

function pokemonPortraitFallback(speciesId: string, name?: string): string {
    const seed = speciesId.split('').reduce((sum, char) => sum + char.charCodeAt(0), 0);
    const hue = seed % 360;
    const label = (name ?? speciesId).slice(0, 2);

    return `data:image/svg+xml;utf8,${encodeURIComponent(`
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 160">
            <rect width="160" height="160" rx="28" fill="hsl(${hue} 42% 24%)"/>
            <circle cx="80" cy="64" r="42" fill="hsl(${hue} 54% 42%)"/>
            <path d="M38 126c12-30 72-30 84 0" fill="hsl(${hue} 48% 34%)"/>
            <text x="80" y="92" text-anchor="middle" font-family="sans-serif" font-size="34" font-weight="700" fill="white">${label}</text>
        </svg>
    `)}`;
}

function getPokemonPortraitSrc(speciesId: string, name?: string): string {
    return POKEMON_IMAGE_BY_ID[speciesId.toLowerCase()] ?? pokemonPortraitFallback(speciesId, name);
}


export default function BattlePage() {
    const navigate = useNavigate();
    const { user } = useAuth();
    const [battleMode] = useState<'ai' | 'player'>(() =>
        sessionStorage.getItem('battleMode') === 'player' ? 'player' : 'ai',
    );
    const [aiLevel] = useState<'lv1' | 'lv2'>(() =>
        sessionStorage.getItem('aiLevel') === 'lv2' ? 'lv2' : 'lv1',
    );
    const [species, setSpecies] = useState<SpeciesData>({});
    const [moves, setMoves] = useState<MoveData>({});
    const [battleState, setBattleState] = useState<BattleStateWire | null>(null);
    const [loading, setLoading] = useState(true);
    const [waiting, setWaiting] = useState(false);    
    const [commandMode, setCommandMode] = useState<'fight' | 'pokemon'>('fight');
    const [focusedTeamSlot, setFocusedTeamSlot] = useState(0);
    const [onlineSnapshot, setOnlineSnapshot] = useState(getOnlineSessionSnapshot());
    const [localPlayerId, setLocalPlayerId] = useState<string>('player');
    const [opponentPlayerId, setOpponentPlayerId] = useState<string>('ai');
    const [statusText, setStatusText] = useState('');
    const [playback, setPlayback] = useState<BattlePlaybackState>(IDLE_PLAYBACK_STATE);
    const [battlePopup, setBattlePopup] = useState<BattlePopup | null>(null);
    const [revealedOpponentSlots, setRevealedOpponentSlots] = useState<Set<number>>(() => new Set());
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
    const playbackRef = useRef(false);
    const popupIdRef = useRef(0);
    const precomputedAiKeyRef = useRef<string | null>(null);

    useEffect(() => {
        battleStateRef.current = battleState;
    }, [battleState]);

    useEffect(() => {
        if (!battleState) {
            return;
        }

        const opponent = battleState.players.find((player) => player.id === opponentPlayerIdRef.current);
        if (!opponent) {
            return;
        }

        setRevealedOpponentSlots((current) => {
            if (current.has(opponent.activeSlot)) {
                return current;
            }
            const next = new Set(current);
            next.add(opponent.activeSlot);
            return next;
        });
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

    useEffect(() => {
        if (
            battleMode !== 'ai'
            || aiLevel !== 'lv2'
            || waiting
            || playback.isPlaying
            || !battleState
        ) {
            return;
        }
        if (
            needsForcedSwitch(battleState, localPlayerIdRef.current)
            || needsForcedSwitch(battleState, opponentPlayerIdRef.current)
        ) {
            return;
        }

        precomputedAiKeyRef.current = precomputeAiAction(
            battleState,
            opponentPlayerIdRef.current,
            VEGA_DEPTH,
            VEGA_PRECOMPUTE_BRANCH_LIMIT,
        );
    }, [aiLevel, battleMode, battleState, playback.isPlaying, waiting]);

    const showBattlePopup = useCallback(async (popup: Omit<BattlePopup, 'id'>) => {
        const id = popupIdRef.current + 1;
        popupIdRef.current = id;
        setBattlePopup({ ...popup, id });
        await wait(BATTLE_POPUP_MS);
        setBattlePopup((current) => (current?.id === id ? null : current));
    }, []);

    const showPopupsFromLogs = useCallback(async (logs: string[], state: BattleStateWire | null) => {
        for (const log of logs) {
            const popup = parseAbilityPopup(
                log,
                state,
                localPlayerIdRef.current,
                opponentPlayerIdRef.current,
            );
            if (!popup) {
                continue;
            }
            await showBattlePopup(popup);
        }
    }, [showBattlePopup]);

    const playBattleResolution = useCallback(async (
        startState: BattleStateWire,
        finalState: BattleStateWire,
        actions: ActionWire[],
    ) => {
        playbackRef.current = true;
        setWaiting(true);
        setStatusText('');
        setPlayback({
            isPlaying: true,
            label: 'ターン処理中',
            faintedCreatureIds: [],
        });

        const stagedState = cloneBattleState(startState);
        stagedState.log = [...startState.log];
        setBattleState(stagedState);

        const newLogs = finalState.log.slice(startState.log.length);
        const actionQueue = orderActionsByBattleLogs(
            actions.length > 0 ? actions : [{ type: 'move', playerId: opponentPlayerIdRef.current } as ActionWire],
            startState,
            newLogs,
            moves,
        );
        const actionLogStarts = actionQueue.map((action) => getActionLogStartIndex(action, startState, newLogs, moves));
        let consumedLogs = 0;
        const animatedFaintIds = new Set<string>();

        await wait(PLAYBACK_STEP_MS);

        for (let actionIndex = 0; actionIndex < actionQueue.length; actionIndex += 1) {
            const action = actionQueue[actionIndex];
            const targetId = action.targetId ?? (
                action.playerId === localPlayerIdRef.current
                    ? opponentPlayerIdRef.current
                    : localPlayerIdRef.current
            );
            const hasLaterTargetAction = actionQueue
                .slice(actionIndex + 1)
                .some((queuedAction) => queuedAction.playerId === targetId);
            const moveType = getMoveType(action.moveId, moves);
            const move = action.moveId ? moves[action.moveId] : undefined;
            const isForceSwitchMove = moveHasEffect(move, 'force_switch');
            const isDamagingForceSwitchMove = isForceSwitchMove && moveHasEffect(move, 'damage');
            const isSelfSwitchMove = moveHasEffect(move, 'self_switch');
            const willSelfSwitch = isSelfSwitchMove && activeSlotChanged(stagedState, finalState, action.playerId);
            const isForcedReplacement = isForcedReplacementAction(action, stagedState);

            setPlayback({
                isPlaying: true,
                label: getActionLabel(action, stagedState, moves, localPlayerIdRef.current),
                attackingPlayerId: action.type === 'switch' ? undefined : action.playerId,
                effectType: moveType,
                faintedCreatureIds: [],
            });

            const nextActionLogStart = actionLogStarts
                .slice(actionIndex + 1)
                .find((index) => Number.isFinite(index));
            const nextLogCount = Math.min(newLogs.length, nextActionLogStart ?? newLogs.length);
            const actionLogStart = consumedLogs;
            const actionLogs = newLogs.slice(consumedLogs, nextLogCount);
            const statusFlash = getStatusFlashFromLogs(actionLogs, stagedState, action.playerId);
            const hasNoEffectStatusLog = actionLogs.some(isStatusNoEffectLog);
            let displayedLogCount = consumedLogs;
            const switchLogIndex = action.type === 'switch'
                ? actionLogs.findIndex((log) => log.includes('を 繰り出した！'))
                : -1;
            const switchLogCount = switchLogIndex >= 0
                ? Math.min(nextLogCount, actionLogStart + switchLogIndex + 1)
                : nextLogCount;
            const initialLogCount = action.type === 'switch'
                ? switchLogCount
                : isForceSwitchMove
                ? Math.min(nextLogCount, consumedLogs + 1)
                : nextLogCount;
            if (initialLogCount > consumedLogs) {
                stagedState.log = [...startState.log, ...newLogs.slice(0, initialLogCount)];
                displayedLogCount = initialLogCount;
                setBattleState(cloneBattleState(stagedState));
            }
            await showPopupsFromLogs(newLogs.slice(consumedLogs, displayedLogCount), stagedState);

            await wait(PLAYBACK_STEP_MS);

            if (action.type === 'switch') {
                const switchInHpDelta = copyFinalActiveCreatureBeforeLoggedHpDelta(
                    stagedState,
                    finalState,
                    action.playerId,
                    actionLogs,
                );
                setBattleState(cloneBattleState(stagedState));
                await wait(isForcedReplacement ? PLAYBACK_FAINT_MS : PLAYBACK_STEP_MS);

                if (nextLogCount > displayedLogCount) {
                    stagedState.log = [...startState.log, ...newLogs.slice(0, nextLogCount)];
                    setBattleState(cloneBattleState(stagedState));
                    await showPopupsFromLogs(newLogs.slice(displayedLogCount, nextLogCount), stagedState);
                    displayedLogCount = nextLogCount;
                }

                if (statusFlash) {
                    setPlayback({
                        isPlaying: true,
                        label: '状態変化を反映中',
                        statusFlashPlayerId: statusFlash.playerId,
                        statusFlashType: statusFlash.statusType,
                        faintedCreatureIds: [],
                    });
                    await wait(PLAYBACK_STEP_MS);
                }

                if (switchInHpDelta !== null) {
                    setPlayback({
                        isPlaying: true,
                        label: switchInHpDelta < 0 ? '場の効果を反映中' : '回復を反映中',
                        damagedPlayerId: switchInHpDelta < 0 ? action.playerId : undefined,
                        effectType: switchInHpDelta < 0 ? 'rock' : undefined,
                        faintedCreatureIds: [],
                    });
                    copyFinalActiveCreature(stagedState, finalState, action.playerId);
                    setBattleState(cloneBattleState(stagedState));
                    await wait(PLAYBACK_HIT_MS);

                    const faintedIds = finalFaintedCreatureIds(stagedState);
                    const newlyFaintedIds = faintedIds.filter((id) => !animatedFaintIds.has(id));
                    if (switchInHpDelta < 0 && newlyFaintedIds.length > 0) {
                        newlyFaintedIds.forEach((id) => animatedFaintIds.add(id));
                        setPlayback((current) => ({
                            ...current,
                            label: 'ひんし処理中',
                            faintedCreatureIds: newlyFaintedIds,
                        }));
                        await wait(PLAYBACK_FAINT_MS);
                    }
                }

                consumedLogs = nextLogCount;
                continue;
            }

            const visualImpactPlayerId = getVisualImpactPlayerId(action, stagedState, targetId, actionLogs, moves);
            setPlayback({
                isPlaying: true,
                label: visualImpactPlayerId === action.playerId
                    ? '効果を反映中'
                    : action.playerId === localPlayerIdRef.current
                        ? '技のエフェクト'
                        : '相手の行動中',
                attackingPlayerId: action.playerId,
                damagedPlayerId: visualImpactPlayerId,
                effectType: moveType,
                statusFlashPlayerId: statusFlash?.playerId,
                statusFlashType: statusFlash?.statusType,
                faintedCreatureIds: [],
            });

            await wait(PLAYBACK_HIT_MS);

            if (isDamagingForceSwitchMove) {
                copyCurrentSlotAfterAction(stagedState, finalState, targetId, actionLogs);
            } else if (!isForceSwitchMove) {
                copyTargetAfterAction(stagedState, finalState, targetId, actionLogs, hasLaterTargetAction);
            } else {
                copyActorAfterOwnAction(stagedState, finalState, action.playerId, actionLogs);
            }
            if ((!isForceSwitchMove || isDamagingForceSwitchMove) && !willSelfSwitch) {
                copyActorAfterOwnAction(stagedState, finalState, action.playerId, actionLogs);
            }
            setBattleState(cloneBattleState(stagedState));

            const faintedIds = finalFaintedCreatureIds(stagedState);
            const newlyFaintedIds = faintedIds.filter((id) => !animatedFaintIds.has(id));
            newlyFaintedIds.forEach((id) => animatedFaintIds.add(id));
            setPlayback((current) => ({
                ...current,
                label: newlyFaintedIds.length > 0
                    ? 'ひんし処理中'
                    : hasNoEffectStatusLog
                        ? '効果を確認中'
                        : statusFlash
                            ? '状態異常を反映中'
                            : 'ダメージ・状態を反映中',
                faintedCreatureIds: newlyFaintedIds,
            }));

            await wait(newlyFaintedIds.length > 0 ? PLAYBACK_FAINT_MS : PLAYBACK_STEP_MS);

            if (isForceSwitchMove && activeSlotChanged(stagedState, finalState, targetId)) {
                const forcedSwitchLogIndex = actionLogs.findIndex((log) => log.includes('を 繰り出した！'));
                const forcedSwitchLogCount = forcedSwitchLogIndex >= 0
                    ? Math.min(nextLogCount, actionLogStart + forcedSwitchLogIndex + 1)
                    : nextLogCount;
                setPlayback({
                    isPlaying: true,
                    label: targetId === localPlayerIdRef.current ? 'あなたが引きずり出されています' : '相手を引きずり出しています',
                    faintedCreatureIds: [],
                });
                await wait(PLAYBACK_STEP_MS);
                if (forcedSwitchLogCount > displayedLogCount) {
                    stagedState.log = [...startState.log, ...newLogs.slice(0, forcedSwitchLogCount)];
                    setBattleState(cloneBattleState(stagedState));
                    await showPopupsFromLogs(newLogs.slice(displayedLogCount, forcedSwitchLogCount), stagedState);
                    displayedLogCount = forcedSwitchLogCount;
                }
                const switchInHpDelta = copyFinalActiveCreatureBeforeLoggedHpDelta(stagedState, finalState, targetId, actionLogs);
                setBattleState(cloneBattleState(stagedState));
                await wait(PLAYBACK_STEP_MS);

                if (switchInHpDelta !== null && switchInHpDelta < 0) {
                    if (nextLogCount > displayedLogCount) {
                        stagedState.log = [...startState.log, ...newLogs.slice(0, nextLogCount)];
                        setBattleState(cloneBattleState(stagedState));
                        await showPopupsFromLogs(newLogs.slice(displayedLogCount, nextLogCount), stagedState);
                        displayedLogCount = nextLogCount;
                    }
                    setPlayback({
                        isPlaying: true,
                        label: '場の効果を反映中',
                        damagedPlayerId: targetId,
                        effectType: 'rock',
                        faintedCreatureIds: [],
                    });
                    copyFinalActiveCreature(stagedState, finalState, targetId);
                    setBattleState(cloneBattleState(stagedState));
                    await wait(PLAYBACK_HIT_MS);
                }
            }

            if (activeSlotChanged(stagedState, finalState, action.playerId)) {
                setPlayback({
                    isPlaying: true,
                    label: action.playerId === localPlayerIdRef.current ? 'あなたが交代しています' : '相手が交代しています',
                    faintedCreatureIds: [],
                });
                copyFinalActiveCreature(stagedState, finalState, action.playerId);
                setBattleState(cloneBattleState(stagedState));
                await wait(PLAYBACK_STEP_MS);
            }

            if (nextLogCount > displayedLogCount) {
                stagedState.log = [...startState.log, ...newLogs.slice(0, nextLogCount)];
                setBattleState(cloneBattleState(stagedState));
                await showPopupsFromLogs(newLogs.slice(displayedLogCount, nextLogCount), stagedState);
            }
            consumedLogs = nextLogCount;
        }

        if (consumedLogs < newLogs.length) {
            const remainingLogs = newLogs.slice(consumedLogs);
            const statusFlash = getStatusFlashFromLogs(remainingLogs, finalState);
            setPlayback((current) => ({
                ...current,
                label: '状態変化を反映中',
                statusFlashPlayerId: statusFlash?.playerId,
                statusFlashType: statusFlash?.statusType,
            }));
            stagedState.log = finalState.log;
            setBattleState(cloneBattleState(stagedState));
            await showPopupsFromLogs(remainingLogs, stagedState);
            await wait(PLAYBACK_STEP_MS);
        }

        setPlayback({
            isPlaying: true,
            label: 'ターン処理完了',
            faintedCreatureIds: [],
        });
        setBattleState(finalState);
        await wait(PLAYBACK_STEP_MS);

        playbackRef.current = false;
        setPlayback(IDLE_PLAYBACK_STATE);
    }, [moves, showPopupsFromLogs]);

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
            battleMode === 'player' &&
            localPlayerIdRef.current === 'host';

        if (shouldUploadStats) {
            battleRecordSavedRef.current = true;
            void uploadGlobalBattleRecord({
                id: battleStatsIdRef.current,
                winner: winnerSide,
                hostDeck: localDeckRef.current,
                guestDeck: opponentDeckRef.current,
                host_user_id: user?.id ?? null,
                guest_user_id: onlineSnapshot.remoteUserId,
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
    }, [battleMode, navigate, onlineSnapshot.remoteUserId, user?.id]);

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
            sendBattleUpdate(nextState, actions);
            await playBattleResolution(currentState, nextState, actions);
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
    }, [finishBattle, playBattleResolution]);

    const resolveForcedSwitch = useCallback(async (action: ActionWire, broadcast: boolean) => {
        const currentState = battleStateRef.current;
        if (!currentState || action.type !== 'switch' || typeof action.slot !== 'number') {
            return;
        }

        try {
            const nextState = replaceFaintedPokemon(currentState, action.playerId, action.slot);
            pendingLocalActionRef.current = null;
            pendingRemoteActionRef.current = null;
            if (broadcast) {
                sendBattleUpdate(nextState, [action]);
            }
            await playBattleResolution(currentState, nextState, [action]);
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
    }, [finishBattle, playBattleResolution]);

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
                setRevealedOpponentSlots(new Set());
                setBattleState(event.state);
                setWaiting(false);
                setStatusText('');
                return;
            }
            if (event.type === 'battle_update') {
                void (async () => {
                    const currentState = battleStateRef.current;
                    if (currentState) {
                        await playBattleResolution(currentState, event.state, event.actions);
                    } else {
                        setBattleState(event.state);
                    }
                    const finished = await finishBattle(event.state);
                    if (!finished) {
                        setWaiting(false);
                        setStatusText('');
                    }
                })();
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
    }, [battleMode, finishBattle, playBattleResolution, resolveForcedSwitch, resolveHostTurn]);

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
                    setRevealedOpponentSlots(new Set());
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
                    setRevealedOpponentSlots(new Set());
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
                setRevealedOpponentSlots(new Set());
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

        const action: ActionWire = {
            type: 'switch',
            playerId: opponentPlayerIdRef.current,
            slot,
        };
        const nextState = replaceFaintedPokemon(battleState, opponentPlayerIdRef.current, slot);
        void (async () => {
            await playBattleResolution(battleState, nextState, [action]);
            const finished = await finishBattle(nextState);
            if (!finished) {
                setWaiting(false);
                setStatusText('');
            }
        })();
    }, [battleMode, battleState, finishBattle, playBattleResolution, waiting]);

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

    const getAiAction = async (state: BattleStateWire): Promise<ActionWire | null> => {
        if (aiLevel === 'lv2') {
            const key = makeAiStateKey(
                state,
                opponentPlayerIdRef.current,
                VEGA_DEPTH,
                VEGA_PRECOMPUTE_BRANCH_LIMIT,
            );
            if (precomputedAiKeyRef.current !== key) {
                precomputedAiKeyRef.current = precomputeAiAction(
                    state,
                    opponentPlayerIdRef.current,
                    VEGA_DEPTH,
                    VEGA_PRECOMPUTE_BRANCH_LIMIT,
                );
            }

            const precomputedAction = await getPrecomputedAiAction(
                key,
                VEGA_PRECOMPUTE_MAX_WAIT_MS,
            );
            if (precomputedAction !== undefined) {
                return precomputedAction ?? getFallbackAiAction(state);
            }

            return await getBestMoveVega(state, opponentPlayerIdRef.current, VEGA_DEPTH)
                ?? getFallbackAiAction(state);
        }
        return await getBestMoveMinimax(state, opponentPlayerIdRef.current, 1) ?? getFallbackAiAction(state);
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
        if (!battleState || waiting || playbackRef.current) return;
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

            const aiAction = await getAiAction(battleState);

if (!aiAction) {
    console.error('AI failed to select action');
    setWaiting(false);
    return;
}

            const currentState = battleState;
            const actions = [playerAction, aiAction];
            const newState = await stepBattle(currentState, actions);
            await playBattleResolution(currentState, newState, actions);
            await finishBattle(newState);
        } catch (err) {
            console.error('Battle step error:', err);
            setStatusText('行動の送信に失敗しました。');
        }

        setWaiting(false);
    };

    const handleSwitch = async (index: number) => {
        if (!battleState || waiting || playbackRef.current) return;
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
                await playBattleResolution(battleState, newState, [playerAction]);
                await finishBattle(newState);
                setWaiting(false);
                setStatusText('');
                return;
            }

            const aiAction = await getAiAction(battleState);

if (!aiAction) {
    console.error('AI failed to select action');
    setWaiting(false);
    return;
}

            const currentState = battleState;
            const actions = [playerAction, aiAction];
            const newState = await stepBattle(currentState, actions);
            await playBattleResolution(currentState, newState, actions);
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
const interactionLocked = waiting || playback.isPlaying;
const battleStatusLabel = playback.isPlaying
    ? playback.label
    : waiting
        ? (statusText || 'ターン処理中')
        : mustSwitch
            ? '交代先を選択中'
            : '行動選択中';
const battleWeatherId = getBattleWeatherId((battleState as BattleStateWithField).field);

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
<div className="flex items-center gap-3">
  <span className={cn(
    'rounded-full border px-3 py-1 text-xs font-semibold',
    interactionLocked
      ? 'border-amber-400/30 bg-amber-400/10 text-amber-200'
      : 'border-emerald-400/30 bg-emerald-400/10 text-emerald-200',
  )}>
    {battleStatusLabel}
  </span>

  <span className="text-sm text-[var(--text-muted)]">
    {battleMode === 'player'
      ? 'VS Player (PeerJS)'
      : aiLevel === 'lv2'
        ? 'VS AI Vega (深さ3)'
        : 'VS AI (Minimax 深さ1)'}
  </span>
</div>
                </div>
                {statusText && (
                    <div className="border-t border-[var(--border)] px-4 py-2 text-center text-sm text-[var(--text-muted)]">
                        {statusText}
                    </div>
                )}
            </header>

            <main className="mx-auto grid h-[calc(100dvh-65px)] min-h-0 w-full max-w-7xl grid-cols-1 gap-4 overflow-hidden px-4 py-4 lg:grid-cols-[minmax(0,1fr)_420px]">
            <section className={cn(
                'battle-weather-base relative flex min-h-0 flex-col gap-2 overflow-hidden rounded-xl',
                getBattleWeatherClass(battleWeatherId),
            )}>
                <BattlePopupToast popup={battlePopup} />
                <div className="flex items-start gap-4">
                    <TeamIndicator team={ai.team} activeSlot={ai.activeSlot} species={species} isPlayer={false} />
                    <PokemonStatus
                        key={aiPokemon.id}
                        creature={aiPokemon}
                        species={aiSpecies}
                        isPlayer={false}
                        isAttacking={playback.attackingPlayerId === opponentPlayerId}
                        isDamaged={playback.damagedPlayerId === opponentPlayerId}
                        isFainting={playback.faintedCreatureIds.includes(aiPokemon.id)}
                        effectType={playback.effectType}
                        statusFlashType={playback.statusFlashPlayerId === opponentPlayerId ? playback.statusFlashType : undefined}
                    />
                </div>
                
                <BattleFieldStatusPanel
                    field={(battleState as BattleStateWithField).field}
                    localPlayerId={localPlayerId}
                    opponentPlayerId={opponentPlayerId}
                />

                <div className="flex items-end gap-4">
                    <TeamIndicator team={player.team} activeSlot={player.activeSlot} species={species} isPlayer={true} />
                    <PokemonStatus
                        key={playerPokemon.id}
                        creature={playerPokemon}
                        species={playerSpecies}
                        isPlayer={true}
                        isAttacking={playback.attackingPlayerId === localPlayerId}
                        isDamaged={playback.damagedPlayerId === localPlayerId}
                        isFainting={playback.faintedCreatureIds.includes(playerPokemon.id)}
                        effectType={playback.effectType}
                        statusFlashType={playback.statusFlashPlayerId === localPlayerId ? playback.statusFlashType : undefined}
                    />
                </div>

                <div className="rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-3">
                <div className="mb-2 grid grid-cols-2 gap-2">
    <button
        onClick={() => setCommandMode('fight')}
        disabled={interactionLocked}
        className={cn(
            'rounded-lg p-2 text-sm font-medium transition-all',
            commandMode === 'fight'
                ? 'bg-[var(--accent)] text-white'
                : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]',
            interactionLocked && 'cursor-not-allowed opacity-50'
        )}
    >
        たたかう
    </button>

    <button
        onClick={() => setCommandMode('pokemon')}
        disabled={interactionLocked}
        className={cn(
            'rounded-lg p-2 text-sm font-medium transition-all',
            commandMode === 'pokemon'
                ? 'bg-[var(--accent)] text-white'
                : 'bg-[var(--surface-3)] text-[var(--text-muted)] hover:bg-[var(--surface-4)]',
            interactionLocked && 'cursor-not-allowed opacity-50'
        )}
    >
        ニキモン
    </button>
</div>
                    {commandMode === 'fight' ? (
                        <div>
                            <div className="mb-2 grid grid-cols-2 gap-2">
                                {playerPokemon.moves.map((moveId) => {
                                    const move = moves[moveId];
                                    const rawPp = playerPokemon.movePp;
                                    const pp = (rawPp instanceof Map ? rawPp.get(moveId) : (rawPp as Record<string, number | undefined>)?.[moveId]) ?? move.pp ?? 10;

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
                                                disabled={interactionLocked || pp === 0}
                                                className={cn(
                                                    'w-full rounded-xl border p-2.5 text-left transition-all',
                                                    interactionLocked || pp === 0
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
                    ) : null}
                </div>
                </section>

                <aside className="min-h-0 rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-3">
            <div ref={logsRef} className="h-full min-h-0 pr-1">
        <BattleLog
            logs={battleState.log}
            currentTurn={battleState.turn}
            className="h-full"
        />
            </div>
        </aside>
        {!playback.isPlaying && (commandMode === 'pokemon' || mustSwitch) && (
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
                    const shownStatuses = visibleStatuses(mon.statuses);
                    const statusLabel = shownStatuses.length > 0
                        ? shownStatuses.map((status) => getStatusLabel(status.id)).join(' / ')
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
                                        const total = stats.hp + stats.atk + stats.def + stats.spa + stats.spd + stats.spe;
                                        const renderBar = (label: string, value: number, max: number) => {
                                            const percentage = Math.min(100, (value / max) * 100);
                                            return (
                                                <div className="grid grid-cols-[64px_1fr_40px] items-center gap-2 text-xs">
                                                    <span className="text-[var(--text-muted)]">{label}</span>
                                                    <div className="relative h-2.5 overflow-hidden rounded-full bg-[var(--surface-4)]">
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
                                                {renderBar('HP', stats.hp, 255)}
                                                {renderBar('攻撃', stats.atk, 255)}
                                                {renderBar('防御', stats.def, 255)}
                                                {renderBar('特攻', stats.spa, 255)}
                                                {renderBar('特防', stats.spd, 255)}
                                                {renderBar('素早さ', stats.spe, 255)}
                                                {renderBar('合計', total, 720)}
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
                                    disabled={isActive || isFainted || interactionLocked}
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
                <div className="mb-2 flex items-center justify-between gap-3">
                    <div className="text-sm font-bold text-[var(--text-primary)]">
                        相手チーム
                    </div>
                    <div className="text-xs text-[var(--text-muted)]">
                        登場済みのみ表示
                    </div>
                </div>

                {ai.team.map((mon, idx) => {
                    const monSpecies = species[mon.speciesId];
                    const isActive = idx === ai.activeSlot;
                    const isFainted = mon.hp <= 0;
                    const isRevealed = isActive || revealedOpponentSlots.has(idx);
                    const hpPercentage = mon.maxHp > 0 ? (mon.hp / mon.maxHp) * 100 : 0;
                    const hpColor = hpPercentage > 50 ? 'bg-emerald-500' : hpPercentage > 20 ? 'bg-amber-500' : 'bg-red-500';
                    const portraitSrc = getPokemonPortraitSrc(mon.speciesId, monSpecies?.name || mon.name);

                    return (
                        <div
                            key={idx}
                            className={cn(
                                'rounded-lg border p-2 text-left transition-all',
                                isRevealed
                                    ? isActive
                                        ? 'border-[var(--accent)] bg-[var(--accent-muted)]'
                                        : 'border-[var(--border)] bg-[var(--surface-3)]'
                                    : 'border-[var(--border)] bg-black/30 opacity-45',
                                isFainted && isRevealed && 'opacity-60'
                            )}
                        >
                            {isRevealed ? (
                                <div className="flex items-center gap-2">
                                    <div className="relative size-12 shrink-0 overflow-hidden rounded-md border border-[var(--border)] bg-[var(--surface-3)]">
                                        <img
                                            src={portraitSrc}
                                            alt={monSpecies?.name || mon.name}
                                            className={cn('size-full object-cover', isFainted && 'grayscale')}
                                        />
                                        {isActive && (
                                            <span className="absolute bottom-0.5 right-0.5 size-2.5 rounded-full border border-[var(--surface-2)] bg-[var(--accent)]" />
                                        )}
                                    </div>
                                    <div className="min-w-0 flex-1">
                                        <div className="truncate text-sm font-semibold text-[var(--text-primary)]">
                                            {monSpecies?.name ?? mon.name}
                                        </div>
                                        <div className="mt-1 flex flex-wrap gap-1">
                                            {(monSpecies?.type ?? mon.types).map((type) => (
                                                <span
                                                    key={type}
                                                    className="rounded-full px-1.5 py-0.5 text-[10px] text-white"
                                                    style={{ backgroundColor: getTypeColor(type) }}
                                                >
                                                    {getTypeLabel(type)}
                                                </span>
                                            ))}
                                        </div>
                                        <div className="mt-1.5 flex items-center gap-2">
                                            <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-[var(--surface-4)]">
                                                <div
                                                    className={cn('h-full transition-all duration-700 ease-out', hpColor)}
                                                    style={{ width: `${hpPercentage}%` }}
                                                />
                                            </div>
                                            <div className="shrink-0 text-[10px] tabular-nums text-[var(--text-muted)]">
                                                {mon.hp}/{mon.maxHp}
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            ) : (
                                <div className="flex min-h-12 items-center gap-2">
                                    <div className="flex size-12 shrink-0 items-center justify-center rounded-md border border-[var(--border)] bg-black/30 text-lg font-bold text-[var(--text-muted)]">
                                        ?
                                    </div>
                                    <div className="min-w-0">
                                        <div className="text-sm font-semibold text-[var(--text-muted)]">未確認</div>
                                        <div className="mt-1 text-xs text-[var(--text-muted)]">情報なし</div>
                                    </div>
                                </div>
                            )}
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
                                    'h-full transition-all duration-700 ease-out',
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

function BattlePopupToast({ popup }: { popup: BattlePopup | null }) {
    if (!popup) {
        return null;
    }

    const positionClass = popup.side === 'opponent'
        ? 'left-14 top-3 battle-popup-slide-opponent'
        : popup.side === 'player'
            ? 'right-4 bottom-48 battle-popup-slide-player'
            : 'right-4 top-1/2 -translate-y-1/2 battle-popup-slide';

    return (
        <div className={cn('pointer-events-none absolute z-30 w-[min(360px,calc(100%-2rem))]', positionClass)}>
            <div className={cn(
                'rounded-xl border px-4 py-3 shadow-2xl backdrop-blur-md',
                popup.tone === 'ability'
                    ? 'border-cyan-300/40 bg-cyan-950/85 text-cyan-50 shadow-cyan-950/40'
                    : 'border-slate-300/30 bg-slate-950/85 text-slate-50 shadow-slate-950/40',
            )}>
                <div className="text-xs font-semibold uppercase tracking-wide text-cyan-200/80">
                    {popup.tone === 'ability' ? 'Ability' : 'Battle'}
                </div>
                <div className="mt-0.5 text-lg font-bold leading-tight">{popup.title}</div>
                <div className="mt-1 text-sm text-white/75">{popup.text}</div>
            </div>
        </div>
    );
}

function PokemonStatus({
    creature,
    species,
    isPlayer,
    isAttacking = false,
    isDamaged = false,
    isFainting = false,
    effectType = 'normal',
    statusFlashType,
}: {
    creature: CreatureStateWire;
    species: SpeciesData[string] | undefined;
    isPlayer: boolean;
    isAttacking?: boolean;
    isDamaged?: boolean;
    isFainting?: boolean;
    effectType?: string;
    statusFlashType?: BattleStatusFlashType;
}) {
    const hpPercentage = creature.maxHp > 0 ? (creature.hp / creature.maxHp) * 100 : 0;
    const hpColor = hpPercentage > 50 ? 'bg-emerald-500' : hpPercentage > 20 ? 'bg-amber-500' : 'bg-red-500';
    const portraitSrc = getPokemonPortraitSrc(creature.speciesId, species?.name || creature.name);
    const typeColor = getTypeColor(effectType);
    const statusFlashColor = statusFlashType ? STATUS_FLASH_COLORS[statusFlashType] : undefined;

    return (
        <div className={cn('flex-1', isPlayer ? 'text-right' : 'text-left')}>
            <div
                className={cn(
                    'relative inline-block min-w-64 rounded-xl border border-[var(--border)] bg-[var(--surface-2)] p-4 transition-all duration-300',
                    isAttacking && (isPlayer ? 'battle-lunge-player' : 'battle-lunge-opponent'),
                    isDamaged && 'battle-shake',
                    isFainting && 'battle-faint',
                )}
            >
                {isDamaged && (
                    <div
                        className="pointer-events-none absolute inset-0 rounded-xl border-2 opacity-70 battle-hit-flash"
                        style={{ borderColor: typeColor, boxShadow: `0 0 28px ${typeColor}` }}
                    />
                )}
                <div className={cn('flex items-center gap-3', isPlayer ? 'flex-row-reverse' : '')}>
                    <div className="relative size-24 shrink-0 overflow-hidden rounded-lg border border-[var(--border)] bg-[var(--surface-3)]">
                        <img
                            src={portraitSrc}
                            alt={species?.name || creature.name}
                            className="size-full object-cover"
                            draggable={false}
                        />
                        {statusFlashColor && (
                            <div
                                className="pointer-events-none absolute inset-0 battle-status-flash"
                                style={{
                                    backgroundColor: statusFlashColor,
                                    boxShadow: `inset 0 0 0 2px ${statusFlashColor}, 0 0 24px ${statusFlashColor}`,
                                }}
                            />
                        )}
                        <div className={cn(
                            'absolute bottom-1 size-3 rounded-full ring-2 ring-[var(--surface-2)]',
                            isPlayer ? 'right-1 bg-blue-400' : 'left-1 bg-red-400',
                        )} />
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
                            className={cn('h-full transition-all duration-1400 ease-out', hpColor)}
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
                    if (stages.accuracy !== 0) displayStages.push({ label: 'めいちゅう', value: stages.accuracy });
                    if (stages.evasion !== 0) displayStages.push({ label: 'かいひ', value: stages.evasion });

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
                        {visibleStatuses(creature.statuses).map((status, i) => (
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
