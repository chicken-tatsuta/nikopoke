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
                this.weights = await response.json() as MlpWeights;
            } catch (error) {
                console.warn('[mlpAI] Failed to load AI weights:', error);
                this.weights = null;
            } finally {
                this.loadPromise = null;
            }
        })();

        return this.loadPromise;
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
            if (mask[index] && value > bestValue) {
                bestValue = value;
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
        this.appendSideFeatures(features, player, opponentActive);
        this.appendSideFeatures(features, opponent, active);
        this.appendBenchFeatures(features, player, opponentActive);
        this.appendBenchFeatures(features, opponent, active);

        for (let slot = 0; slot < 4; slot += 1) {
            const moveId = active?.moves[slot];
            const move = moveId ? moves[moveId] : undefined;

            if (!moveId || !move || !opponentActive) {
                features.push(0, 0, 0, 0, 0);
                continue;
            }

            const maxPp = Math.max(move.pp ?? 10, 1);
            const remainingPp = Math.max(active?.movePp?.[moveId] ?? maxPp, 0);
            const category = move.category;
            const isPhysical = category === 'physical';
            const isSpecial = category === 'special';
            const powerNorm = isPhysical || isSpecial ? (move.power ?? 0) / 150 : 0;
            const typeEffectiveness = this.getTypeEffectiveness(move.type, opponentActive.types) / 4;

            features.push(
                remainingPp / maxPp,
                powerNorm,
                typeEffectiveness,
                isPhysical ? 1 : 0,
                category === 'status' ? 1 : 0,
            );
        }

        return features;
    }

    private appendSideFeatures(
        features: number[],
        player?: PlayerStateWire,
        opponentActive?: CreatureStateWire,
    ): void {
        const active = player?.team[player.activeSlot];
        if (!player || !active || !opponentActive) {
            features.push(...Array(49).fill(0));
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
    ): void {
        if (!player || !opponentActive) {
            features.push(0, 0, 0, 0, 0, 0);
            return;
        }

        const bench = player.team
            .map((pokemon, index) => ({ pokemon, index }))
            .filter(({ pokemon, index }) => index !== player.activeSlot && pokemon.hp > 0)
            .map(({ pokemon }) => pokemon);

        for (let slot = 0; slot < 2; slot += 1) {
            const pokemon = bench[slot];
            if (!pokemon) {
                features.push(0, 0, 0);
                continue;
            }

            const hpRatio = Math.max(0, Math.min(1, pokemon.hp / Math.max(pokemon.maxHp, 1)));
            const attackingType = opponentActive.types[0] ?? '';
            const typeEffVsOpp = this.getTypeEffectiveness(attackingType, pokemon.types) / 4;
            features.push(hpRatio, 1, typeEffVsOpp);
        }
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
