import type { SimulationSummary } from "./result";
import type { ProjectedWinningPlans, SimulatorInput } from "./simulator";

export type MatchupWorkerRequest =
  | {
      requestId: number;
      type: "run-single";
      left: SimulatorInput;
      right: SimulatorInput;
      battleCount: number;
    }
  | {
      requestId: number;
      type: "run-right-item-candidates";
      left: SimulatorInput;
      right: SimulatorInput;
      rightItemIds: string[];
      battleCount: number;
    }
  | {
      requestId: number;
      type: "build-winning-plans";
      left: SimulatorInput;
      right: SimulatorInput;
      itemId?: string;
    };

export type MatchupWorkerRequestPayload =
  | Omit<Extract<MatchupWorkerRequest, { type: "run-single" }>, "requestId">
  | Omit<Extract<MatchupWorkerRequest, { type: "run-right-item-candidates" }>, "requestId">
  | Omit<Extract<MatchupWorkerRequest, { type: "build-winning-plans" }>, "requestId">;

export type MatchupWorkerResponse =
  | {
      requestId: number;
      type: "run-single-result";
      summary: SimulationSummary;
      strongestWinningPlans: ProjectedWinningPlans;
    }
  | {
      requestId: number;
      type: "run-right-item-candidates-result";
      summary: SimulationSummary;
      byRightItem: Array<{
        itemId: string;
        summary: SimulationSummary;
      }>;
    }
  | {
      requestId: number;
      type: "build-winning-plans-result";
      itemId?: string;
      strongestWinningPlans: ProjectedWinningPlans;
    }
  | {
      requestId: number;
      type: "error";
      message: string;
    };
