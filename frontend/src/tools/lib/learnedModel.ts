import learnedModelJson from "../data/learnedMoveModel.json";
import { extractMoveFeatures, extractStateFeatures, type SparseFeatureMap } from "./learnedFeatures";
import type { SimBattlePokemon, SimMove } from "./simulator";

export type LinearLearnedModel = {
  label: string;
  intercept: number;
  weights: Record<string, number>;
  outputScale?: number;
  featureVersion?: number;
  sampleCount?: number;
  trainLoss?: number;
  validationLoss?: number;
};

export type LearnedModelBundle = {
  version: number;
  trainedAt: string | null;
  moveModel: LinearLearnedModel | null;
  stateModel: LinearLearnedModel | null;
};

const learnedModel = learnedModelJson as LearnedModelBundle;

const scoreFeatures = (
  model: LinearLearnedModel | null,
  features: SparseFeatureMap
) => {
  if (!model) {
    return 0;
  }

  let total = model.intercept;

  for (const [key, value] of Object.entries(features)) {
    const weight = model.weights[key];
    if (weight !== undefined) {
      total += weight * value;
    }
  }

  return total * (model.outputScale ?? 1);
};

const clampLearnedBias = (value: number) => {
  const maxBias = 6000;
  if (!Number.isFinite(value)) {
    return 0;
  }

  return Math.max(-maxBias, Math.min(maxBias, value));
};

export const hasLearnedMoveModel = () =>
  Boolean(learnedModel.moveModel && Object.keys(learnedModel.moveModel.weights).length > 0);

export const hasLearnedStateValueModel = () =>
  Boolean(learnedModel.stateModel && Object.keys(learnedModel.stateModel.weights).length > 0);

export const getLearnedMoveBias = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon,
  move: SimMove
) => {
  if (!hasLearnedMoveModel()) {
    return 0;
  }

  return clampLearnedBias(
    scoreFeatures(learnedModel.moveModel, extractMoveFeatures(attacker, defender, move))
  );
};

export const getLearnedStateValueBias = (
  attacker: SimBattlePokemon,
  defender: SimBattlePokemon
) => {
  if (!hasLearnedStateValueModel()) {
    return 0;
  }

  return clampLearnedBias(
    scoreFeatures(learnedModel.stateModel, extractStateFeatures(attacker, defender))
  );
};

export const getLearnedModelMetadata = () => learnedModel;
