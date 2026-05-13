import type { BattleStateWire, ActionWire, CreatureStateWire, PlayerStateWire } from './engine';
import type { MoveData } from '../types/pokemon';

interface MlpWeights {
    w1: number[][];
    b1: number[];
    w2: number[][];
    b2: number[];
    w3: number[][];
    b3: number[];
    w4: number[][];
    b4: number[];
}

const TYPES_LIST = [
    'bug', 'dark', 'dragon', 'electric', 'fairy', 'fighting',
    'fire', 'flying', 'ghost', 'grass', 'ground', 'ice',
    'normal', 'poison', 'psychic', 'rock', 'steel', 'water',
] as const;

const FEATURE_SIZE = 166;
const CONFIRMED_KO_BONUS = 2.0;
const FIRST_CONFIRMED_KO_BONUS = 1.0;
const IMMEDIATE_DEATH_SWITCH_PENALTY = -3.0;
const SAFE_SWITCH_WHEN_THREATENED_BONUS = 1.0;

const TYPE_EFFECTIVENESS: Record<string, Partial<Record<string, number>>> = {
    normal: { rock: 0.5, ghost: 0, steel: 0.5 },
    fire: { fire: 0.5, water: 0.5, grass: 2, ice: 2, bug: 2, rock: 0.5, dragon: 0.5, steel: 2 },
    water: { fire: 2, water: 0.5, grass: 0.5, ground: 2, rock: 2, dragon: 0.5 },
    electric: { water: 2, electric: 0.5, grass: 0.5, ground: 0, flying: 2, dragon: 0.5 },
    grass: { fire: 0.5, water: 2, grass: 0.5, poison: 0.5, ground: 2, flying: 0.5, bug: 0.5, rock: 2, dragon: 0.5, steel: 0.5 },
    ice: { fire: 0.5, water: 0.5, grass: 2, ice: 0.5, ground: 2, flying: 2, dragon: 2, steel: 0.5 },
    fighting: { normal: 2, ice: 2, poison: 0.5, flying: 0.5, psychic: 0.5, bug: 0.5, rock: 2, ghost: 0, dark: 2, steel: 2, fairy: 0.5 },
    poison: { grass: 2, poison: 0.5, ground: 0.5, rock: 0.5, ghost: 0.5, steel: 0, fairy: 2 },
    ground: { fire: 2, electric: 2, grass: 0.5, poison: 2, flying: 0, bug: 0.5, rock: 2, steel: 2 },
    flying: { electric: 0.5, grass: 2, fighting: 2, bug: 2, rock: 0.5, steel: 0.5 },
    psychic: { fighting: 2, poison: 2, psychic: 0.5, dark: 0, steel: 0.5 },
    bug: { fire: 0.5, grass: 2, fighting: 0.5, poison: 0.5, flying: 0.5, psychic: 2, ghost: 0.5, dark: 2, steel: 0.5, fairy: 0.5 },
    rock: { fire: 2, ice: 2, fighting: 0.5, ground: 0.5, flying: 2, bug: 2, steel: 0.5 },
    ghost: { normal: 0, psychic: 2, ghost: 2, dark: 0.5 },
    dragon: { dragon: 2, steel: 0.5, fairy: 0 },
    dark: { fighting: 0.5, psychic: 2, ghost: 2, dark: 0.5, fairy: 0.5 },
    steel: { fire: 0.5, water: 0.5, electric: 0.5, ice: 2, rock: 2, steel: 0.5, fairy: 2 },
    fairy: { fire: 0.5, fighting: 2, poison: 0.5, dragon: 2, dark: 2, steel: 0.5 },
};

class MlpAI {
    private weights: MlpWeights | null = null;
    private loadPromise: Promise<void> | null = null;

    async load(): Promise<void> {
        if (this.weights) return;
        if (this.loadPromise) return this.loadPromise;

        this.loadPromise = (async () => {
            try {
                const response = await fetch('/ai_weights.json');
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                const weights = await response.json() as MlpWeights;
                if (!this.hasExpectedShape(weights)) {
                    throw new Error('unexpected MLP weight shape');
                }
                this.weights = weights;
            } catch (error) {
                console.warn('[mlpAI] Failed to load AI weights:', error);
                this.weights = null;
            } finally {
                this.loadPromise = null;
            }
        })();

        return this.loadPromise;
    }

    private hasExpectedShape(weights: MlpWeights): boolean {
        return Array.isArray(weights.w1)
            && weights.w1.length === 128
            && weights.w1.every((row) => Array.isArray(row) && row.length === FEATURE_SIZE);
    }

    isReady(): boolean {
        return this.weights !== null;
    }

    getBestAction(
        state: BattleStateWire,
        playerId: string,
        moves: MoveData,
    ): ActionWire | null {
        if (!this.weights) return null;

        const features = this.extractFeatures(state, playerId, moves);
        const logits = this.forward(features);
        const mask = this.actionMask(state, playerId, moves);

        let bestIndex = -1;
        let bestValue = -Infinity;
        logits.forEach((value, index) => {
            if (!mask[index]) return;
            const action = this.actionFromSlot(state, playerId, index);
            const adjustedValue = value + (action ? this.ruleBonus(state, playerId, action, moves) : 0);
            if (adjustedValue > bestValue) {
                bestValue = adjustedValue;
                bestIndex = index;
            }
        });

        if (bestIndex < 0) return null;
        return this.actionFromSlot(state, playerId, bestIndex);
    }

    private extractFeatures(
        state: BattleStateWire,
        playerId: string,
        moves: MoveData,
    ): number[] {
        const player = state.players.find((candidate) => candidate.id === playerId);
        const opponent = state.players.find((candidate) => candidate.id !== playerId);
        const active = player?.team[player.activeSlot];
        const opponentActive = opponent?.team[opponent.activeSlot];

        const features: number[] = [];
        this.appendSideFeatures(features, player, opponentActive, moves);
        this.appendSideFeatures(features, opponent, active, moves);
        this.appendBenchFeatures(features, player, opponentActive, moves);
        this.appendBenchFeatures(features, opponent, active, moves);

        for (let slot = 0; slot < 4; slot += 1) {
            const moveId = active?.moves[slot];
            const move = moveId ? moves[moveId] : undefined;

            if (!moveId || !move || !opponentActive) {
                features.push(0, 0, 0, 0, 0, 0, 0, 0, 0);
                continue;
            }

            const maxPp = Math.max(move.pp ?? 10, 1);
            const remainingPp = this.remainingPp(active, moveId, maxPp);
            const ppRatio = remainingPp / maxPp;
            const category = move.category;
            const isPhysical = category === 'physical';
            const isSpecial = category === 'special';
            const powerNorm = isPhysical || isSpecial ? (move.power ?? 0) / 150 : 0;
            const typeEffectiveness = this.getTypeEffectiveness(move.type, opponentActive.types) / 4;
            const expectedHit = Math.min(1, powerNorm * typeEffectiveness);
            const priorityNorm = Math.max(-1, Math.min(1, (move.priority ?? 0) / 5));

            features.push(
                ppRatio,
                remainingPp <= 0 ? 1 : 0,
                ppRatio > 0 && ppRatio <= 0.25 ? 1 : 0,
                powerNorm,
                typeEffectiveness,
                expectedHit,
                priorityNorm,
                isPhysical ? 1 : 0,
                category === 'status' ? 1 : 0,
            );
        }

        return features;
    }

    private ruleBonus(
        state: BattleStateWire,
        playerId: string,
        action: ActionWire,
        moves: MoveData,
    ): number {
        const player = state.players.find((candidate) => candidate.id === playerId);
        const opponent = state.players.find((candidate) => candidate.id !== playerId);
        const active = player?.team[player.activeSlot];
        const opponentActive = opponent?.team[opponent.activeSlot];
        if (!player || !active || !opponentActive) return 0;

        if (action.type === 'move') {
            const moveId = action.moveId;
            const move = moveId ? moves[moveId] : undefined;
            if (!moveId || !move) return 0;
            const maxPp = this.maxPp(moveId, moves);
            if (this.remainingPp(active, moveId, maxPp) <= 0) return -9999;
            if (!this.isReliableDamageMove(move)) return 0;

            if (this.estimatedMinDamage(active, opponentActive, move) >= opponentActive.hp) {
                let bonus = CONFIRMED_KO_BONUS;
                if (this.movesBefore(active, opponentActive, move)) {
                    bonus += FIRST_CONFIRMED_KO_BONUS;
                }
                return bonus;
            }
            return 0;
        }

        if (action.type === 'switch') {
            const target = action.slot === undefined ? undefined : player.team[action.slot];
            if (!target) return 0;
            if (this.canActiveConfirmedKo(opponentActive, target, moves)) {
                return IMMEDIATE_DEATH_SWITCH_PENALTY;
            }
            if (this.canActiveConfirmedKo(opponentActive, active, moves)) {
                return SAFE_SWITCH_WHEN_THREATENED_BONUS;
            }
        }

        return 0;
    }

    private appendSideFeatures(
        features: number[],
        player?: PlayerStateWire,
        opponentActive?: CreatureStateWire,
        moves?: MoveData,
    ): void {
        const active = player?.team[player.activeSlot];
        if (!player || !active || !opponentActive) {
            features.push(...Array(53).fill(0));
            return;
        }

        const hpRatio = Math.max(0, Math.min(1, active.hp / Math.max(active.maxHp, 1)));
        const aliveCount = player.team.filter((pokemon) => pokemon.hp > 0).length;
        const hpSum = player.team.reduce((sum, pokemon) => {
            return sum + Math.max(0, Math.min(1, pokemon.hp / Math.max(pokemon.maxHp, 1)));
        }, 0);

        features.push(
            hpRatio,
            active.stages.atk / 6,
            active.stages.def / 6,
            active.stages.spa / 6,
            active.stages.spd / 6,
            active.stages.spe / 6,
            this.hasStatus(active, ['burn', 'burned']) ? 1 : 0,
            this.hasStatus(active, ['sleep', 'asleep']) ? 1 : 0,
            this.hasStatus(active, ['poison', 'toxic', 'badly_poisoned']) ? 1 : 0,
            this.hasStatus(active, ['paralysis', 'paralyze', 'paralyzed']) ? 1 : 0,
            aliveCount / 3,
            hpSum / 3,
        );
        this.appendTypeOnehot(features, active.types[0]);
        this.appendTypeOnehot(features, active.types[1]);

        const speed = Math.max(active.speed, 0);
        const opponentSpeed = Math.max(opponentActive.speed, 0);
        features.push(speed / (speed + opponentSpeed + 1e-8));
        this.appendPpSummary(features, active, moves);
    }

    private appendTypeOnehot(features: number[], typeName?: string): void {
        for (const candidate of TYPES_LIST) {
            features.push(typeName === candidate ? 1 : 0);
        }
    }

    private appendBenchFeatures(
        features: number[],
        player?: PlayerStateWire,
        opponentActive?: CreatureStateWire,
        moves?: MoveData,
    ): void {
        if (!player || !opponentActive) {
            features.push(...Array(12).fill(0));
            return;
        }

        const bench = player.team
            .map((pokemon, index) => ({ pokemon, index }))
            .filter(({ pokemon, index }) => index !== player.activeSlot && pokemon.hp > 0)
            .map(({ pokemon }) => pokemon);

        for (let slot = 0; slot < 2; slot += 1) {
            const pokemon = bench[slot];
            if (!pokemon) {
                features.push(0, 0, 0, 0, 0, 0);
                continue;
            }

            const hpRatio = Math.max(0, Math.min(1, pokemon.hp / Math.max(pokemon.maxHp, 1)));
            const attackingType = opponentActive.types[0] ?? '';
            const typeEffVsOpp = this.getTypeEffectiveness(attackingType, pokemon.types) / 4;
            const bestOffense = this.bestOffenseScore(pokemon, opponentActive, moves);
            const speed = Math.max(pokemon.speed, 0);
            const opponentSpeed = Math.max(opponentActive.speed, 0);
            const speedRatio = speed / (speed + opponentSpeed + 1e-8);

            features.push(
                hpRatio,
                1,
                typeEffVsOpp,
                bestOffense,
                speedRatio,
                speed > opponentSpeed ? 1 : 0,
            );
        }
    }

    private appendPpSummary(features: number[], creature: CreatureStateWire, moves?: MoveData): void {
        const moveIds = creature.moves.slice(0, 4);
        if (moveIds.length === 0) {
            features.push(0, 0, 0, 0);
            return;
        }

        let usable = 0;
        let totalRatio = 0;
        let empty = 0;
        let low = 0;

        for (const moveId of moveIds) {
            const maxPp = this.maxPp(moveId, moves);
            const remaining = this.remainingPp(creature, moveId, maxPp);
            const ratio = remaining / maxPp;
            totalRatio += ratio;
            if (remaining > 0) usable += 1;
            if (remaining <= 0) empty += 1;
            if (remaining > 0 && ratio <= 0.25) low += 1;
        }

        const count = Math.max(moveIds.length, 1);
        features.push(
            usable / count,
            totalRatio / count,
            empty / count,
            low / count,
        );
    }

    private bestOffenseScore(attacker: CreatureStateWire, defender: CreatureStateWire, moves?: MoveData): number {
        let best = 0;
        for (const moveId of attacker.moves.slice(0, 4)) {
            const move = moves?.[moveId];
            if (!move || move.category === 'status') continue;
            const maxPp = this.maxPp(moveId, moves);
            if (this.remainingPp(attacker, moveId, maxPp) <= 0) continue;

            const powerNorm = Math.max(move.power ?? 0, 0) / 150;
            const typeEffectiveness = this.getTypeEffectiveness(move.type, defender.types) / 4;
            best = Math.max(best, Math.min(1, powerNorm * typeEffectiveness));
        }
        return best;
    }

    private canActiveConfirmedKo(
        attacker: CreatureStateWire,
        defender: CreatureStateWire,
        moves: MoveData,
    ): boolean {
        return attacker.moves.slice(0, 4).some((moveId) => {
            const move = moves[moveId];
            const maxPp = this.maxPp(moveId, moves);
            return Boolean(move)
                && this.remainingPp(attacker, moveId, maxPp) > 0
                && this.isReliableDamageMove(move)
                && this.estimatedMinDamage(attacker, defender, move) >= defender.hp;
        });
    }

    private isReliableDamageMove(move: MoveData[string]): boolean {
        return move.category !== 'status'
            && (move.power ?? 0) > 0
            && (move.accuracy ?? 1) >= 1;
    }

    private estimatedMinDamage(
        attacker: CreatureStateWire,
        defender: CreatureStateWire,
        move: MoveData[string],
    ): number {
        const power = Math.max(move.power ?? 0, 0);
        if (power <= 0) return 0;

        const isSpecial = move.category === 'special';
        const attackStat = (isSpecial ? attacker.spAttack : attacker.attack)
            * this.statStageMultiplier(isSpecial ? attacker.stages.spa : attacker.stages.atk);
        const defenseStat = Math.max(
            (isSpecial ? defender.spDefense : defender.defense)
                * this.statStageMultiplier(isSpecial ? defender.stages.spd : defender.stages.def),
            1,
        );
        const base = (((2 * attacker.level / 5 + 2) * power * attackStat / defenseStat) / 50) + 2;
        const stab = attacker.types.includes(move.type) ? 1.5 : 1;
        const effectiveness = this.getTypeEffectiveness(move.type, defender.types);
        return Math.max(1, Math.floor(base * stab * effectiveness * 0.85));
    }

    private statStageMultiplier(stage: number): number {
        const clamped = Math.max(-6, Math.min(6, stage));
        return clamped >= 0 ? (2 + clamped) / 2 : 2 / (2 - clamped);
    }

    private movesBefore(
        attacker: CreatureStateWire,
        defender: CreatureStateWire,
        move: MoveData[string],
    ): boolean {
        const priority = move.priority ?? 0;
        if (priority > 0) return true;
        if (priority < 0) return false;
        return this.modifiedSpeed(attacker) >= this.modifiedSpeed(defender);
    }

    private modifiedSpeed(creature: CreatureStateWire): number {
        return Math.max(creature.speed, 0) * this.statStageMultiplier(creature.stages.spe);
    }

    private maxPp(moveId: string, moves?: MoveData): number {
        return Math.max(moves?.[moveId]?.pp ?? 10, 1);
    }

    private remainingPp(creature: CreatureStateWire, moveId: string, maxPp: number): number {
        return Math.max(creature.movePp?.[moveId] ?? maxPp, 0);
    }

    private forward(x: number[]): number[] {
        if (!this.weights) return [];

        const h1 = this.relu(this.matVecAdd(this.weights.w1, x, this.weights.b1));
        const h2 = this.relu(this.matVecAdd(this.weights.w2, h1, this.weights.b2));
        const h3 = this.relu(this.matVecAdd(this.weights.w3, h2, this.weights.b3));
        return this.matVecAdd(this.weights.w4, h3, this.weights.b4);
    }

    private relu(x: number[]): number[] {
        return x.map((value) => Math.max(0, value));
    }

    private matVecAdd(w: number[][], x: number[], b: number[]): number[] {
        return w.map((row, rowIndex) => {
            return row.reduce((sum, weight, index) => sum + weight * (x[index] ?? 0), 0) + (b[rowIndex] ?? 0);
        });
    }

    private actionMask(state: BattleStateWire, playerId: string, moves: MoveData): boolean[] {
        const mask = [false, false, false, false, false, false];
        const player = state.players.find((candidate) => candidate.id === playerId);
        const active = player?.team[player.activeSlot];
        if (!player || !active) return mask;

        const forcedSwitch = active.hp <= 0 || this.hasStatus(active, ['pending_switch']);
        if (!forcedSwitch) {
            active.moves.slice(0, 4).forEach((moveId, index) => {
                const maxPp = moves[moveId]?.pp ?? 10;
                const remainingPp = active.movePp?.[moveId] ?? maxPp;
                mask[index] = Boolean(moves[moveId]) && remainingPp > 0;
            });
        }

        this.benchSlots(player).slice(0, 2).forEach((_, benchIndex) => {
            mask[4 + benchIndex] = true;
        });

        return mask;
    }

    private actionFromSlot(state: BattleStateWire, playerId: string, slot: number): ActionWire | null {
        const player = state.players.find((candidate) => candidate.id === playerId);
        const opponent = state.players.find((candidate) => candidate.id !== playerId);
        if (!player) return null;

        if (slot >= 0 && slot <= 3) {
            const moveId = player.team[player.activeSlot]?.moves[slot];
            if (!moveId) return null;
            return {
                type: 'move',
                playerId,
                moveId,
                targetId: opponent?.id,
            };
        }

        if (slot >= 4 && slot <= 5) {
            const switchSlot = this.benchSlots(player)[slot - 4];
            if (switchSlot === undefined) return null;
            return {
                type: 'switch',
                playerId,
                slot: switchSlot,
            };
        }

        return null;
    }

    private benchSlots(player: PlayerStateWire): number[] {
        return player.team
            .map((pokemon, index) => ({ pokemon, index }))
            .filter(({ pokemon, index }) => index !== player.activeSlot && pokemon.hp > 0)
            .map(({ index }) => index);
    }

    private hasStatus(creature: CreatureStateWire, ids: string[]): boolean {
        return creature.statuses.some((status) => ids.includes(status.id));
    }

    private getTypeEffectiveness(moveType?: string, targetTypes?: string[]): number {
        if (!moveType || !targetTypes?.length) return 1;

        return targetTypes.reduce((multiplier, targetType) => {
            return multiplier * (TYPE_EFFECTIVENESS[moveType]?.[targetType] ?? 1);
        }, 1);
    }
}

export const mlpAI = new MlpAI();
