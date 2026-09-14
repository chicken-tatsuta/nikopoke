/* tslint:disable */
/* eslint-disable */

export function createBattleState(players: any): any;

export function createCreature(species_id: string, options: any): any;

export function getBestMoveMCTS(state: any, player_id: string, iterations: number): any;

export function getBestMoveMinimax(state: any, player_id: string, depth: number): any;

export function getBestMoveVega(state: any, player_id: string, depth: number): any;

export function getBestMoveVegaIterative(state: any, player_id: string, max_depth: number, node_budget: number): any;

export function getBestMoveVegaWithBranch(state: any, player_id: string, depth: number, branch_limit: number): any;

export function isBattleOver(state: any): boolean;

export function replaceFaintedPokemon(state: any, player_id: string, slot: number): any;

export function stepBattle(state: any, actions: any, options: any): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly createBattleState: (a: number, b: number) => void;
  readonly createCreature: (a: number, b: number, c: number, d: number) => void;
  readonly getBestMoveMCTS: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly getBestMoveMinimax: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly getBestMoveVega: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly getBestMoveVegaIterative: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly getBestMoveVegaWithBranch: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
  readonly isBattleOver: (a: number, b: number) => void;
  readonly replaceFaintedPokemon: (a: number, b: number, c: number, d: number, e: number) => void;
  readonly stepBattle: (a: number, b: number, c: number, d: number) => void;
  readonly __wbindgen_export: (a: number, b: number) => number;
  readonly __wbindgen_export2: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export3: (a: number) => void;
  readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
