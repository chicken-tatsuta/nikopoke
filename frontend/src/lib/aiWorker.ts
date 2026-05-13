import init, {
    getBestMoveVegaWithBranch as wasmGetBestMoveVegaWithBranch,
} from './engine-rust/engine_rust.js';
import type { ActionWire, BattleStateWire } from './engine';

type AiWorkerRequest = {
    id: number;
    kind: 'vega';
    state: BattleStateWire;
    playerId: string;
    depth: number;
    branchLimit: number;
};

type AiWorkerResponse = {
    id: number;
    action: ActionWire | null;
    elapsedMs: number;
    error?: string;
};

let initPromise: Promise<void> | null = null;

function ensureInit(): Promise<void> {
    initPromise ??= init().then(() => undefined);
    return initPromise;
}

self.onmessage = (event: MessageEvent<AiWorkerRequest>) => {
    const request = event.data;
    void (async () => {
        const startedAt = performance.now();
        try {
            await ensureInit();
            const action = wasmGetBestMoveVegaWithBranch(
                request.state,
                request.playerId,
                request.depth,
                request.branchLimit,
            ) as ActionWire | null;
            const response: AiWorkerResponse = {
                id: request.id,
                action,
                elapsedMs: performance.now() - startedAt,
            };
            self.postMessage(response);
        } catch (error) {
            const response: AiWorkerResponse = {
                id: request.id,
                action: null,
                elapsedMs: performance.now() - startedAt,
                error: error instanceof Error ? error.message : String(error),
            };
            self.postMessage(response);
        }
    })();
};
