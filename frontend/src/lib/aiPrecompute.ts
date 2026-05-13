import type { ActionWire, BattleStateWire } from './engine';

type WorkerResponse = {
    id: number;
    action: ActionWire | null;
    elapsedMs: number;
    error?: string;
};

type PendingEntry = {
    id: number;
    key: string;
    startedAt: number;
    promise: Promise<ActionWire | null>;
    resolve: (action: ActionWire | null) => void;
};

let worker: Worker | null = null;
let nextRequestId = 1;
let pending: PendingEntry | null = null;
const completed = new Map<string, ActionWire | null>();

function getWorker(): Worker {
    worker ??= new Worker(new URL('./aiWorker.ts', import.meta.url), { type: 'module' });
    worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
        const response = event.data;
        if (!pending || pending.id !== response.id) return;
        if (response.error) {
            console.warn('[aiPrecompute] worker failed:', response.error);
        }
        completed.set(pending.key, response.action);
        pending.resolve(response.action);
        pending = null;
    };
    worker.onerror = (event) => {
        console.warn('[aiPrecompute] worker error:', event.message);
        pending?.resolve(null);
        pending = null;
    };
    return worker;
}

function resetWorker(): void {
    worker?.terminate();
    worker = null;
}

export function makeAiStateKey(
    state: BattleStateWire,
    playerId: string,
): string {
    return JSON.stringify({
        v: 2,
        playerId,
        turn: state.turn,
        players: state.players.map((player) => ({
            id: player.id,
            activeSlot: player.activeSlot,
            team: player.team.map((creature) => ({
                id: creature.id,
                speciesId: creature.speciesId,
                level: creature.level,
                types: creature.types,
                moves: creature.moves,
                ability: creature.ability,
                item: creature.item,
                hp: creature.hp,
                maxHp: creature.maxHp,
                stages: creature.stages,
                statuses: creature.statuses,
                movePp: creature.movePp,
                attack: creature.attack,
                defense: creature.defense,
                spAttack: creature.spAttack,
                spDefense: creature.spDefense,
                speed: creature.speed,
            })),
        })),
        field: state.field,
    });
}

export function precomputeAiAction(
    state: BattleStateWire,
    playerId: string,
    maxDepth: number,
    nodeBudget: number,
): string {
    const key = makeAiStateKey(state, playerId);
    if (completed.has(key) || pending?.key === key) return key;
    if (pending && pending.key !== key) {
        pending.resolve(null);
        pending = null;
        resetWorker();
    }

    const id = nextRequestId++;
    const promise = new Promise<ActionWire | null>((resolve) => {
        pending = { id, key, startedAt: performance.now(), promise: Promise.resolve(null), resolve };
    });
    if (pending) {
        pending.promise = promise;
    }

    getWorker().postMessage({
        id,
        kind: 'vega-iterative',
        state,
        playerId,
        maxDepth,
        nodeBudget,
    });
    return key;
}

export async function getPrecomputedAiAction(
    key: string,
    maxWaitMs: number,
): Promise<ActionWire | null | undefined> {
    if (completed.has(key)) {
        return completed.get(key) ?? null;
    }
    if (!pending || pending.key !== key) {
        return undefined;
    }

    const elapsed = performance.now() - pending.startedAt;
    const remaining = Math.max(0, maxWaitMs - elapsed);
    if (remaining <= 0) {
        pending.resolve(null);
        pending = null;
        resetWorker();
        return undefined;
    }

    const result = await Promise.race([
        pending.promise,
        new Promise<undefined>((resolve) => window.setTimeout(resolve, remaining)),
    ]);
    if (result === undefined && pending?.key === key) {
        pending.resolve(null);
        pending = null;
        resetWorker();
    }
    return result;
}
