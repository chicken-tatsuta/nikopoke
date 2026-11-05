/// <reference lib="webworker" />

import { moveData } from "../data/catalog";
import type { MatchupWorkerRequest, MatchupWorkerResponse } from "../lib/matchupWorkerTypes";
import { summarizeSimulationResults } from "../lib/result";
import {
  buildStrongestWinningPlans,
  simulateMatchup,
  simulateMatchupByRightItemCandidates,
  type SimMove,
} from "../lib/simulator";

const MOVES = moveData as SimMove[];
const MOVE_MAP = new Map(MOVES.map((move) => [move.id, move]));

const workerScope = self as DedicatedWorkerGlobalScope;

workerScope.onmessage = (event: MessageEvent<MatchupWorkerRequest>) => {
  const request = event.data;

  try {
    switch (request.type) {
      case "run-single": {
        const result = simulateMatchup({
          left: request.left,
          right: request.right,
          moveMap: MOVE_MAP,
          battleCount: request.battleCount,
        });

        const response: MatchupWorkerResponse = {
          requestId: request.requestId,
          type: "run-single-result",
          summary: summarizeSimulationResults(result.battles),
          strongestWinningPlans: buildStrongestWinningPlans(
            request.left,
            request.right,
            MOVE_MAP
          ),
        };

        workerScope.postMessage(response);
        return;
      }

      case "run-right-item-candidates": {
        const result = simulateMatchupByRightItemCandidates({
          left: request.left,
          right: request.right,
          rightItemIds: request.rightItemIds,
          moveMap: MOVE_MAP,
          battleCount: request.battleCount,
        });

        const response: MatchupWorkerResponse = {
          requestId: request.requestId,
          type: "run-right-item-candidates-result",
          summary: summarizeSimulationResults(result.battles),
          byRightItem: result.byRightItem.map((entry) => ({
            itemId: entry.itemId,
            summary: summarizeSimulationResults(entry.battles),
          })),
        };

        workerScope.postMessage(response);
        return;
      }

      case "build-winning-plans": {
        const response: MatchupWorkerResponse = {
          requestId: request.requestId,
          type: "build-winning-plans-result",
          itemId: request.itemId,
          strongestWinningPlans: buildStrongestWinningPlans(
            request.left,
            request.right,
            MOVE_MAP
          ),
        };

        workerScope.postMessage(response);
        return;
      }
    }
  } catch (error) {
    const response: MatchupWorkerResponse = {
      requestId: request.requestId,
      type: "error",
      message: error instanceof Error ? error.message : "Unknown worker error",
    };

    workerScope.postMessage(response);
  }
};

export {};
