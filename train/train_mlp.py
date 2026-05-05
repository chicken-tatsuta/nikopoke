import json
import os
import random
import subprocess
import tempfile
from multiprocessing import Pool
from pathlib import Path

import numpy as np


POPULATION_SIZE = 24
GENERATIONS = 30
GAMES_PER_MATCH = 5
SURVIVORS = 6
MUTATION_SCALE = 0.05
HOF_MAX_SIZE = 15
HOF_GAMES = 5
EVALUATE_TIMEOUT_SECONDS = 120
RANDOM_TEAM_MATCH_RATE = 0.10

TRAIN_DIR = Path(__file__).resolve().parent
DATA_DIR = TRAIN_DIR.parent / "engine-rust/data"
BINARY_PATH = TRAIN_DIR.parent / "engine-rust/target/release/self-play-export"
OUTPUT_PATH = TRAIN_DIR.parent / "frontend/public/ai_weights.json"

W1 = (128, 130)
B1 = (128,)
W2 = (64, 128)
B2 = (64,)
W3 = (32, 64)
B3 = (32,)
W4 = (6, 32)
B4 = (6,)

hall_of_fame = []


def load_species_data():
    with (DATA_DIR / "species.json").open(encoding="utf-8") as f:
        return json.load(f)


def load_learnset_data():
    with (DATA_DIR / "learnsets.json").open(encoding="utf-8") as f:
        return json.load(f)


SPECIES_DATA = load_species_data()
LEARNSET_DATA = load_learnset_data()
SPECIES_IDS = sorted(SPECIES_DATA.keys())


def random_weights(scale=0.1):
    return {
        "w1": np.random.randn(*W1) * scale,
        "b1": np.zeros(B1),
        "w2": np.random.randn(*W2) * scale,
        "b2": np.zeros(B2),
        "w3": np.random.randn(*W3) * scale,
        "b3": np.zeros(B3),
        "w4": np.random.randn(*W4) * scale,
        "b4": np.zeros(B4),
    }


def random_team():
    species_sample = random.sample(SPECIES_IDS, 3)
    team = []
    for sid in species_sample:
        learnable = LEARNSET_DATA.get(sid, [])
        moves = random.sample(learnable, min(4, len(learnable)))
        while len(moves) < 4 and learnable:
            moves.append(random.choice(learnable))
        team.append({"species_id": sid, "moves": moves[:4]})
    return team


def mutate_team(team, p=0.15):
    team = [dict(member) for member in team]
    for index in range(len(team)):
        if random.random() < p:
            new_sid = random.choice(SPECIES_IDS)
            learnable = LEARNSET_DATA.get(new_sid, [])
            moves = random.sample(learnable, min(4, len(learnable)))
            while len(moves) < 4 and learnable:
                moves.append(random.choice(learnable))
            team[index] = {"species_id": new_sid, "moves": moves[:4]}
        else:
            team[index] = dict(team[index])
            team[index]["moves"] = list(team[index]["moves"])
            if random.random() < p:
                learnable = LEARNSET_DATA.get(team[index]["species_id"], [])
                if learnable:
                    slot = random.randrange(4)
                    while len(team[index]["moves"]) < 4:
                        team[index]["moves"].append(random.choice(learnable))
                    team[index]["moves"][slot] = random.choice(learnable)
    return team


def weights_to_jsonable(weights):
    return {key: value.tolist() for key, value in weights.items()}


def individual_to_jsonable(ind):
    return {
        "weights": weights_to_jsonable(ind["weights"]),
        "team": copy_team(ind["team"]),
    }


def save_weights(weights, path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        json.dump(weights_to_jsonable(weights), f, separators=(",", ":"))


def expected_shapes():
    return {
        "w1": W1,
        "b1": B1,
        "w2": W2,
        "b2": B2,
        "w3": W3,
        "b3": B3,
        "w4": W4,
        "b4": B4,
    }


def weights_file_has_current_shape(path):
    if not path.exists():
        return False
    try:
        with path.open(encoding="utf-8") as f:
            weights = json.load(f)
    except (OSError, json.JSONDecodeError):
        return False

    for key, shape in expected_shapes().items():
        value = np.asarray(weights.get(key))
        if value.shape != shape:
            return False
    return True


def ensure_placeholder_weights():
    if weights_file_has_current_shape(OUTPUT_PATH):
        return

    state = np.random.get_state()
    np.random.seed(42)
    save_weights(random_weights(scale=0.1), OUTPUT_PATH)
    np.random.set_state(state)


def save_team(team, path):
    with path.open("w", encoding="utf-8") as f:
        json.dump(team, f, separators=(",", ":"))


def evaluate_pair(ind_a, ind_b, games, seed_offset, use_random_teams=False):
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        path_wa = temp_path / "wa.json"
        path_wb = temp_path / "wb.json"
        path_ta = temp_path / "ta.json"
        path_tb = temp_path / "tb.json"

        save_weights(ind_a["weights"], path_wa)
        save_weights(ind_b["weights"], path_wb)
        save_team(ind_a["team"], path_ta)
        save_team(ind_b["team"], path_tb)

        cmd = [
            str(BINARY_PATH),
            "--weight-a",
            str(path_wa),
            "--weight-b",
            str(path_wb),
            "--games",
            str(games),
            "--seed",
            str(seed_offset),
        ]
        if not use_random_teams:
            cmd.extend(["--team-a", str(path_ta), "--team-b", str(path_tb)])

        try:
            result = subprocess.run(
                cmd,
                check=True,
                capture_output=True,
                text=True,
                timeout=EVALUATE_TIMEOUT_SECONDS,
            )
            parsed = json.loads(result.stdout)
            return parsed["wins_a"], parsed["wins_b"]
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired, json.JSONDecodeError) as exc:
            print(f"evaluate_pair failed seed={seed_offset}: {exc}", flush=True)
            return 0, 0


def evaluate_batch(matches):
    if not matches:
        return []

    with tempfile.TemporaryDirectory() as temp_dir:
        batch_path = Path(temp_dir) / "batch.json"
        payload = []
        for match in matches:
            payload.append(
                {
                    "weights_a": weights_to_jsonable(match["ind_a"]["weights"]),
                    "weights_b": weights_to_jsonable(match["ind_b"]["weights"]),
                    "team_a": None if match["use_random_teams"] else copy_team(match["ind_a"]["team"]),
                    "team_b": None if match["use_random_teams"] else copy_team(match["ind_b"]["team"]),
                    "games": match["games"],
                    "seed": match["seed"],
                }
            )

        with batch_path.open("w", encoding="utf-8") as f:
            json.dump(payload, f, separators=(",", ":"))

        try:
            result = subprocess.run(
                [str(BINARY_PATH), "--batch", str(batch_path)],
                check=True,
                capture_output=True,
                text=True,
                timeout=EVALUATE_TIMEOUT_SECONDS * len(matches),
            )
            return json.loads(result.stdout)
        except (subprocess.CalledProcessError, subprocess.TimeoutExpired, json.JSONDecodeError) as exc:
            seeds = ",".join(str(match["seed"]) for match in matches)
            print(f"evaluate_batch failed seeds={seeds}: {exc}", flush=True)
            return [{"wins_a": 0, "wins_b": 0, "draws": 0} for _ in matches]


def _evaluate_worker(args):
    idx, ind, population, current_hof, seed_base = args
    rng = random.Random(seed_base + idx * 9999)
    matches = []
    opponents = [i for i in range(len(population)) if i != idx]
    for match_no, opp_idx in enumerate(rng.sample(opponents, min(3, len(opponents)))):
        matches.append(
            {
                "ind_a": ind,
                "ind_b": population[opp_idx],
                "games": GAMES_PER_MATCH,
                "seed": 42 + idx * 1000 + match_no * 100 + opp_idx,
                "use_random_teams": rng.random() < RANDOM_TEAM_MATCH_RATE,
            }
        )
    if current_hof:
        for match_no, hof_opp in enumerate(rng.sample(current_hof, min(2, len(current_hof)))):
            matches.append(
                {
                    "ind_a": ind,
                    "ind_b": hof_opp,
                    "games": HOF_GAMES,
                    "seed": 42 + idx * 2000 + match_no,
                    "use_random_teams": False,
                }
            )
    return sum(result["wins_a"] for result in evaluate_batch(matches))


def tournament(population):
    seed_base = random.randint(0, 10 ** 9)
    args = [
        (idx, ind, population, list(hall_of_fame), seed_base)
        for idx, ind in enumerate(population)
    ]
    num_workers = min(os.cpu_count() or 1, len(population))
    with Pool(num_workers) as pool:
        scores = pool.map(_evaluate_worker, args)
    return sorted(zip(scores, population), key=lambda item: item[0], reverse=True)


def update_hof(ranked_individuals):
    best = ranked_individuals[0][1]
    hall_of_fame.append(copy_individual(best))
    if len(hall_of_fame) > HOF_MAX_SIZE:
        hall_of_fame.pop(0)


def save_individual(individual, generation):
    save_weights(individual["weights"], OUTPUT_PATH)

    team_output_path = OUTPUT_PATH.parent / "ai_team.json"
    with team_output_path.open("w", encoding="utf-8") as f:
        json.dump(individual["team"], f, indent=2)

    checkpoint_dir = TRAIN_DIR / "checkpoints"
    checkpoint_dir.mkdir(parents=True, exist_ok=True)
    checkpoint_path = checkpoint_dir / f"gen_{generation:03d}.json"
    with checkpoint_path.open("w", encoding="utf-8") as f:
        json.dump(individual_to_jsonable(individual), f, separators=(",", ":"))


def mutate(ind):
    return {
        "weights": {
            key: value + np.random.randn(*value.shape) * MUTATION_SCALE
            for key, value in ind["weights"].items()
        },
        "team": mutate_team(ind["team"]),
    }


def copy_weights(weights):
    return {key: value.copy() for key, value in weights.items()}


def copy_team(team):
    return [{"species_id": member["species_id"], "moves": list(member["moves"])} for member in team]


def copy_individual(ind):
    return {
        "weights": copy_weights(ind["weights"]),
        "team": copy_team(ind["team"]),
    }


def evolve(survivors):
    next_population = [copy_individual(ind) for ind in survivors]

    while len(next_population) < POPULATION_SIZE:
        parent = random.choice(survivors)
        next_population.append(mutate(parent))

    return next_population


def main():
    ensure_placeholder_weights()
    np.random.seed(42)
    random.seed(42)

    if not os.path.exists(BINARY_PATH):
        raise FileNotFoundError(
            f"{BINARY_PATH} not found. Build it first with: cargo build --release --bin self-play-export"
        )

    population = [
        {"weights": random_weights(), "team": random_team()}
        for _ in range(POPULATION_SIZE)
    ]
    best_individual = copy_individual(population[0])

    for generation in range(1, GENERATIONS + 1):
        ranked = tournament(population)
        best_score, best_individual = ranked[0]
        update_hof(ranked)
        hof_matches = min(2, len(hall_of_fame) - 1) if generation > 1 else 0
        total_games = 3 * GAMES_PER_MATCH + hof_matches * HOF_GAMES
        print(f"Generation {generation}/{GENERATIONS}: {best_score}/{total_games} wins", flush=True)
        save_individual(best_individual, generation)

        survivors = [ind for _, ind in ranked[:SURVIVORS]]
        population = evolve(survivors)

    save_individual(best_individual, GENERATIONS)


if __name__ == "__main__":
    main()
