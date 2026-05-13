#!/usr/bin/env python3
"""Hybrid move YAML generator.

Pipeline:
1) Build deterministic base move DSL from PokeAPI + CSV.
2) Refine only `steps` (and optional tags/critRate) with Gemini using move descriptions.
3) Emit `moves.yaml` and a report JSON.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import re
import time
import unicodedata
from pathlib import Path
from typing import Any

import requests
import yaml

POKEAPI_LIST_URL = "https://pokeapi.co/api/v2/move?limit=2000&offset=0"
GEMINI_URL_BASE = "https://generativelanguage.googleapis.com/v1beta/models"
SUPPORTED_TYPES = {
    "normal",
    "fire",
    "water",
    "electric",
    "grass",
    "ice",
    "fighting",
    "poison",
    "ground",
    "flying",
    "psychic",
    "bug",
    "rock",
    "ghost",
    "dragon",
    "dark",
    "steel",
    "fairy",
}
SUPPORTED_CATEGORY = {"physical", "special", "status"}
NAME_ALIASES_JA = {
    "しっぽぎり": "しっぽきり",
    "ゴットバード": "ゴッドバード",
    "ごっとばーど": "ゴッドバード",
    "はめつのめがい": "はめつのねがい",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate moves.yaml with PokeAPI base + Gemini refinement")
    parser.add_argument("--csv", default="data/2期生男子種族値 - 技一覧.csv")
    parser.add_argument("--output", default="data/moves.yaml")
    parser.add_argument("--report", default="data/move_hybrid_report.json")
    parser.add_argument("--cache-dir", default="data/cache/pokeapi/move")
    parser.add_argument("--model", default="gemini-2.0-flash")
    parser.add_argument("--delay-ms", type=int, default=80)
    parser.add_argument("--gemini-delay-ms", type=int, default=250)
    parser.add_argument("--gemini-retries", type=int, default=2)
    parser.add_argument("--gemini-timeout-sec", type=int, default=60)
    parser.add_argument("--max-moves", type=int, default=0, help="0 means all")
    parser.add_argument("--no-gemini", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--strict-cache", action="store_true", help="Do not hit network on cache miss")
    return parser.parse_args()


def repo_roots() -> tuple[Path, Path]:
    cwd = Path.cwd()
    if (cwd / "engine-rust").exists():
        return cwd, cwd / "engine-rust"
    if cwd.name == "engine-rust":
        return cwd.parent, cwd
    return cwd, cwd


def resolve_path(engine_root: Path, path_str: str) -> Path:
    path = Path(path_str)
    return path if path.is_absolute() else engine_root / path


def normalize_name(name: str) -> str:
    raw = unicodedata.normalize("NFKC", name).strip().lower()
    out = []
    for ch in raw:
        if ch.isspace():
            continue
        if ch in "・･/／-−ー―–—~〜()（）[]【】":
            continue
        out.append(ch)
    return "".join(out)


def to_snake_id(name: str) -> str:
    move_id = name.lower().replace("-", "_").replace(" ", "_")
    move_id = move_id.replace("'", "").replace("’", "")
    return move_id


def read_csv_records(csv_path: Path) -> list[dict[str, str]]:
    records: list[dict[str, str]] = []
    with csv_path.open("r", encoding="utf-8", newline="") as f:
        reader = csv.reader(f)
        headers = next(reader, None)
        if not headers:
            return records
        for row in reader:
            if len(row) < 9:
                continue
            name = row[0].strip()
            if not name:
                continue
            records.append(
                {
                    "name": name,
                    "type": row[2].strip(),
                    "power": row[3].strip(),
                    "accuracy": row[4].strip(),
                    "pp": row[5].strip(),
                    "category": row[6].strip(),
                    "contact": row[7].strip(),
                    "effect_csv": row[8].strip(),
                }
            )
    return records


def request_json(url: str, timeout_sec: int = 30) -> dict[str, Any]:
    res = requests.get(url, timeout=timeout_sec)
    res.raise_for_status()
    return res.json()


def fetch_move_list(delay_ms: int) -> list[dict[str, Any]]:
    url = POKEAPI_LIST_URL
    items: list[dict[str, Any]] = []
    while url:
        data = request_json(url)
        items.extend(data.get("results", []))
        url = data.get("next")
        if url and delay_ms > 0:
            time.sleep(delay_ms / 1000.0)
    return items


def load_or_fetch_move_detail(
    cache_dir: Path,
    move_item: dict[str, Any],
    strict_cache: bool,
    delay_ms: int,
) -> dict[str, Any] | None:
    move_id = extract_id_from_url(move_item.get("url", "")) or move_item.get("name", "")
    if not move_id:
        return None
    cache_file = cache_dir / f"{move_id}.json"
    if cache_file.exists():
        return json.loads(cache_file.read_text(encoding="utf-8"))

    if strict_cache:
        return None

    detail = request_json(move_item["url"])
    cache_file.parent.mkdir(parents=True, exist_ok=True)
    cache_file.write_text(json.dumps(detail, ensure_ascii=False, indent=2), encoding="utf-8")
    if delay_ms > 0:
        time.sleep(delay_ms / 1000.0)
    return detail


def extract_id_from_url(url: str) -> str | None:
    parts = [p for p in url.strip("/").split("/") if p]
    return parts[-1] if parts else None


def is_japanese_lang(code: str) -> bool:
    return code in {"ja", "ja-Hrkt", "ja-kanji"}


def japanese_names(move_detail: dict[str, Any]) -> list[str]:
    out = []
    for n in move_detail.get("names", []):
        lang = (((n or {}).get("language") or {}).get("name")) or ""
        if is_japanese_lang(lang):
            val = (n or {}).get("name")
            if isinstance(val, str) and val:
                out.append(val)
    return out


def choose_name(move_detail: dict[str, Any]) -> str:
    names = move_detail.get("names", [])
    for target in ("ja-Hrkt", "ja", "ja-kanji"):
        for n in names:
            lang = (((n or {}).get("language") or {}).get("name")) or ""
            if lang == target and n.get("name"):
                return n["name"]
    return move_detail.get("name", "")


def choose_description(move_detail: dict[str, Any], csv_effect: str) -> str:
    for entry in move_detail.get("effect_entries", []):
        lang = (((entry or {}).get("language") or {}).get("name")) or ""
        if is_japanese_lang(lang):
            short = entry.get("short_effect")
            if short:
                return str(short).replace("\n", " ")
            full = entry.get("effect")
            if full:
                return str(full).replace("\n", " ")
    for entry in move_detail.get("flavor_text_entries", []):
        lang = (((entry or {}).get("language") or {}).get("name")) or ""
        if is_japanese_lang(lang) and entry.get("flavor_text"):
            return str(entry["flavor_text"]).replace("\n", " ")
    return csv_effect


def map_ailment(name: str) -> str | None:
    mapping = {
        "paralysis": "paralysis",
        "burn": "burn",
        "poison": "poison",
        "bad-poison": "bad_poison",
        "sleep": "sleep",
        "freeze": "freeze",
        "confusion": "confusion",
        "flinch": "flinch",
    }
    return mapping.get(name)


def map_stat(name: str) -> str | None:
    mapping = {
        "attack": "atk",
        "defense": "def",
        "special-attack": "spa",
        "special-defense": "spd",
        "speed": "spe",
        "accuracy": "accuracy",
        "evasion": "evasion",
    }
    return mapping.get(name)


def parse_condition_target(move_detail: dict[str, Any]) -> str:
    target_name = (((move_detail.get("target") or {}).get("name")) or "").lower()
    return "self" if target_name.startswith("user") else "target"


def build_base_steps(move_detail: dict[str, Any]) -> tuple[list[dict[str, Any]], list[str]]:
    steps: list[dict[str, Any]] = []
    reasons: list[str] = []
    move_id = to_snake_id(move_detail.get("name", ""))
    power = move_detail.get("power")
    accuracy = move_detail.get("accuracy")
    accuracy_f = (accuracy / 100.0) if isinstance(accuracy, int) else None
    meta = move_detail.get("meta") or {}
    target = parse_condition_target(move_detail)

    if (meta.get("category") or {}).get("name") == "ohko":
        effect = {"type": "ohko"}
        if isinstance(accuracy_f, float):
            effect["baseAccuracy"] = accuracy_f
        return [effect], reasons

    min_hits = meta.get("min_hits")
    max_hits = meta.get("max_hits")
    if isinstance(power, int):
        damage = {"type": "damage", "power": power}
        if isinstance(accuracy_f, float):
            damage["accuracy"] = accuracy_f
        if isinstance(min_hits, int) or isinstance(max_hits, int):
            mi = min_hits if isinstance(min_hits, int) else 2
            ma = max_hits if isinstance(max_hits, int) else mi
            steps.append({"type": "repeat", "times": {"min": mi, "max": ma}, "steps": [damage]})
        else:
            steps.append(damage)

    if not isinstance(power, int):
        healing = meta.get("healing")
        if isinstance(healing, int) and healing > 0:
            steps.append({"type": "damage_ratio", "ratioMaxHp": -(healing / 100.0), "target": "self"})

    drain = meta.get("drain")
    if isinstance(drain, int) and drain != 0 and isinstance(power, int):
        ratio = abs(drain) / 100.0
        if drain > 0:
            steps.append({"type": "damage_ratio", "ratioMaxHp": -ratio, "target": "self"})
        else:
            steps.append({"type": "damage_ratio", "ratioMaxHp": ratio, "target": "self"})
    ailment_name = (meta.get("ailment") or {}).get("name")
    if ailment_name in {"trap"} and (
        isinstance(meta.get("min_turns"), int) or isinstance(meta.get("max_turns"), int)
    ):
        reasons.append("Multi-turn trapping/binding effects are not fully supported")

    flinch = meta.get("flinch_chance")
    if isinstance(flinch, int) and flinch > 0:
        steps.append(
            {
                "type": "chance",
                "p": flinch / 100.0,
                "then": [{"type": "apply_status", "statusId": "flinch", "target": "target"}],
            }
        )

    ailment = ailment_name
    ailment_chance = meta.get("ailment_chance")
    mapped_ailment = map_ailment(ailment) if isinstance(ailment, str) else None
    if mapped_ailment:
        apply = {"type": "apply_status", "statusId": mapped_ailment, "target": target}
        if isinstance(ailment_chance, int) and 0 < ailment_chance < 100:
            steps.append({"type": "chance", "p": ailment_chance / 100.0, "then": [apply]})
        else:
            steps.append(apply)
    elif isinstance(ailment, str) and ailment not in {"none", ""}:
        reasons.append(f"Unsupported ailment: {ailment}")

    changes = move_detail.get("stat_changes") or []
    stages: dict[str, int] = {}
    for c in changes:
        key = map_stat(((c.get("stat") or {}).get("name")) or "")
        val = c.get("change")
        if key and isinstance(val, int):
            stages[key] = val
    if stages:
        stat_target = target
        if target == "target" and isinstance(power, int) and all(v < 0 for v in stages.values()):
            stat_target = "self"
        stat_step = {"type": "modify_stage", "target": stat_target, "stages": stages}
        stat_chance = meta.get("stat_chance")
        if isinstance(stat_chance, int) and 0 < stat_chance < 100:
            steps.append({"type": "chance", "p": stat_chance / 100.0, "then": [stat_step]})
        else:
            steps.append(stat_step)

    category_name = ((meta.get("category") or {}).get("name")) or ""
    if isinstance(category_name, str) and "force-switch" in category_name:
        steps.append({"type": "force_switch"})

    add_special_step_overrides(move_id, steps)
    special = special_steps(move_id)
    if special is not None:
        return special, []

    if not steps:
        reasons.append("No supported effects inferred")
        steps.append({"type": "manual", "manualReason": "No supported effects inferred"})
    elif reasons:
        steps.append({"type": "manual", "manualReason": "; ".join(reasons)})

    return steps, reasons


def add_special_step_overrides(move_id: str, steps: list[dict[str, Any]]) -> None:
    if move_id == "headlong_rush":
        has_damage = any(step.get("type") == "damage" for step in steps)
        has_stage = any(step.get("type") == "modify_stage" for step in steps)
        if has_damage and not has_stage:
            steps.append({"type": "modify_stage", "target": "self", "stages": {"def": -1, "spd": -1}})
    if move_id in {"flip_turn", "u_turn", "volt_switch"}:
        has_damage = any(step.get("type") == "damage" for step in steps)
        has_switch = any(step.get("type") == "self_switch" for step in steps)
        if has_damage and not has_switch:
            steps.append({"type": "self_switch"})


def special_steps(move_id: str) -> list[dict[str, Any]] | None:
    if move_id in {"protect", "detect", "baneful_bunker"}:
        return [{"type": "protect"}]
    if move_id == "endure":
        return [{"type": "endure"}]

    if move_id == "yawn":
        return [{"type": "apply_status", "statusId": "yawn", "target": "target", "data": {"turns": 1}}]

    if move_id == "encore":
        return [{"type": "lock_move", "target": "target", "duration": 3, "data": {"mode": "force_last_move"}}]

    if move_id == "wish":
        return [
            {
                "type": "delay",
                "target": "self",
                "turns": 1,
                "steps": [{"type": "damage_ratio", "ratioMaxHp": -0.5, "target": "self"}],
            }
        ]

    if move_id in {"baton_pass", "teleport"}:
        return [{"type": "self_switch"}]

    if move_id == "substitute":
        return [
            {"type": "damage_ratio", "ratioMaxHp": 0.25, "target": "self"},
            {"type": "apply_status", "statusId": "substitute", "target": "self"},
        ]

    if move_id == "rest":
        return [
            {"type": "cure_all_status", "target": "self"},
            {"type": "damage_ratio", "ratioMaxHp": -1.0, "target": "self"},
            {"type": "apply_status", "statusId": "sleep", "target": "self", "duration": 2},
        ]

    if move_id in {"metronome", "copycat", "sleep_talk"}:
        return [{"type": "random_move", "pool": "self_moves"}]

    if move_id in {"trick_room", "toxic_spikes", "stealth_rock", "spikes", "sticky_web"}:
        map_duration = {"trick_room": 5}
        map_stack = {"toxic_spikes": True, "spikes": True}
        step: dict[str, Any] = {"type": "apply_field_status", "statusId": move_id}
        if move_id in map_duration:
            step["duration"] = map_duration[move_id]
        if move_id in map_stack:
            step["stack"] = map_stack[move_id]
        return [step]

    field_status_moves = {
        "sunny_day": ("sun", 5),
        "rain_dance": ("rain", 5),
        "sandstorm": ("sandstorm", 5),
        "grassy_terrain": ("grassy_terrain", 5),
        "electric_terrain": ("electric_terrain", 5),
        "psychic_terrain": ("psychic_terrain", 5),
        "misty_terrain": ("misty_terrain", 5),
        "tailwind": ("tailwind", 4),
        "light_screen": ("light_screen", 5),
        "reflect": ("reflect", 5),
        "aurora_veil": ("aurora_veil", 5),
    }
    if move_id in field_status_moves:
        status_id, duration = field_status_moves[move_id]
        return [{"type": "apply_field_status", "statusId": status_id, "duration": duration}]

    return None


def build_base_move(csv_record: dict[str, str], move_detail: dict[str, Any]) -> tuple[dict[str, Any], list[str]]:
    base_steps, reasons = build_base_steps(move_detail)
    move_name_en = move_detail.get("name", "")
    move_type = (((move_detail.get("type") or {}).get("name")) or "").lower()
    category = (((move_detail.get("damage_class") or {}).get("name")) or "").lower()
    accuracy = move_detail.get("accuracy")
    power = move_detail.get("power")
    pp = move_detail.get("pp")
    priority = move_detail.get("priority")
    meta = move_detail.get("meta") or {}
    crit_rate = meta.get("crit_rate")

    tags: list[str] = []
    if csv_record.get("contact") == "接触":
        tags.append("contact")

    move_obj: dict[str, Any] = {
        "id": to_snake_id(move_name_en),
        "name": choose_name(move_detail),
        "type": move_type,
        "category": category,
        "pp": pp if isinstance(pp, int) else None,
        "power": power if isinstance(power, int) else None,
        "accuracy": (accuracy / 100.0) if isinstance(accuracy, int) else None,
        "priority": priority if isinstance(priority, int) else 0,
        "description": choose_description(move_detail, csv_record.get("effect_csv", "")),
        "steps": base_steps,
        "tags": tags,
    }
    if isinstance(crit_rate, int) and crit_rate > 0:
        move_obj["critRate"] = crit_rate
    return move_obj, reasons


def strip_code_fence(text: str) -> str:
    block = re.search(r"```(?:json)?\s*(.*?)```", text, flags=re.DOTALL)
    return block.group(1).strip() if block else text.strip()


def build_gemini_prompt(move_obj: dict[str, Any], csv_record: dict[str, str], move_detail: dict[str, Any]) -> str:
    effect_en = ""
    for e in move_detail.get("effect_entries", []):
        if (((e.get("language") or {}).get("name")) or "") == "en":
            effect_en = (e.get("short_effect") or e.get("effect") or "").replace("\n", " ")
            break
    return f"""あなたはポケモンバトルDSLの修正器です。
以下のベースJSONはPokeAPIの構造データから作ったものです。`steps`の精度を上げるために、説明文を使って必要な修正だけ行ってください。

制約:
- `id`, `name`, `type`, `category`, `pp`, `power`, `accuracy`, `priority`, `description` は変更しない
- 返すのは JSON object のみ
- `steps` は配列で必須
- 追加で返してよいのは `tags`, `critRate`
- engineで未対応の複雑効果は `{{"type":"manual","manualReason":"..."}}` に落とす
- 攻撃技なら先頭はなるべく `damage`

CSV効果文:
{csv_record.get("effect_csv", "")}

PokeAPI英語効果文:
{effect_en}

ベースJSON:
{json.dumps(move_obj, ensure_ascii=False, indent=2)}
"""


def build_retry_prompt(base_prompt: str, previous_error: str, attempt: int) -> str:
    return f"""{base_prompt}

前回エラー(再試行 {attempt} 回目):
{previous_error}

上記エラーを解消するように JSON を修正して再出力してください。
"""


def classify_gemini_error(err: Exception | str, validation: bool = False) -> str:
    if validation:
        return "schema_mismatch"
    if isinstance(err, json.JSONDecodeError):
        return "json_parse"
    if isinstance(err, requests.exceptions.Timeout):
        return "timeout"
    if isinstance(err, requests.exceptions.HTTPError):
        code = getattr(err.response, "status_code", None)
        if code == 429:
            return "rate_limited"
        return f"http_{code}" if code else "http_error"
    if isinstance(err, requests.exceptions.RequestException):
        return "request_error"
    return "unknown"


def should_retry_gemini(category: str) -> bool:
    return category in {"json_parse", "schema_mismatch", "timeout", "rate_limited", "request_error"}


def gemini_refine_once(
    prompt: str,
    move_obj: dict[str, Any],
    api_key: str,
    model: str,
    timeout_sec: int,
) -> tuple[dict[str, Any], dict[str, Any] | None]:
    url = f"{GEMINI_URL_BASE}/{model}:generateContent?key={api_key}"
    body = {
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": {
            "temperature": 0.1,
            "topK": 40,
            "topP": 0.95,
            "maxOutputTokens": 4096,
            "responseMimeType": "application/json",
        },
    }
    try:
        res = requests.post(url, json=body, timeout=timeout_sec)
        res.raise_for_status()
        payload = res.json()
        candidate = (((payload.get("candidates") or [{}])[0].get("content") or {}).get("parts") or [{}])[0]
        raw = candidate.get("text") or ""
        parsed = json.loads(strip_code_fence(raw))
        if not isinstance(parsed, dict):
            return move_obj, {"category": "schema_mismatch", "reason": "Gemini output is not an object"}
        refined = merge_refined(move_obj, parsed)
        err = validate_move(refined)
        if err:
            return move_obj, {"category": "schema_mismatch", "reason": f"Validation failed: {err}"}
        return refined, None
    except Exception as e:  # noqa: BLE001
        return move_obj, {"category": classify_gemini_error(e), "reason": str(e)}


def gemini_refine(
    move_obj: dict[str, Any],
    csv_record: dict[str, str],
    move_detail: dict[str, Any],
    api_key: str,
    model: str,
    retries: int,
    timeout_sec: int,
) -> tuple[dict[str, Any], dict[str, Any] | None]:
    base_prompt = build_gemini_prompt(move_obj, csv_record, move_detail)
    last_error: dict[str, Any] | None = None
    max_attempts = retries + 1

    for attempt in range(1, max_attempts + 1):
        prompt = base_prompt
        if attempt > 1 and last_error:
            prompt = build_retry_prompt(base_prompt, str(last_error.get("reason")), attempt - 1)
        refined, err = gemini_refine_once(prompt, move_obj, api_key, model, timeout_sec)
        if err is None:
            return refined, None
        last_error = {"attempt": attempt, **err}
        if attempt >= max_attempts or not should_retry_gemini(str(err.get("category", ""))):
            break
        wait_sec = min(6.0, 1.2 * attempt)
        time.sleep(wait_sec)

    return move_obj, last_error


def merge_refined(base: dict[str, Any], refined: dict[str, Any]) -> dict[str, Any]:
    out = dict(base)
    if isinstance(refined.get("steps"), list):
        out["steps"] = refined["steps"]
    if isinstance(refined.get("tags"), list):
        tags = [t for t in refined["tags"] if isinstance(t, str)]
        out["tags"] = sorted(set((out.get("tags") or []) + tags))
    if isinstance(refined.get("critRate"), int):
        out["critRate"] = refined["critRate"]
    return out


def validate_move(move_obj: dict[str, Any]) -> str | None:
    required = ["id", "name", "type", "category", "pp", "power", "accuracy", "priority", "steps"]
    for key in required:
        if key not in move_obj:
            return f"missing key: {key}"
    if move_obj.get("type") not in SUPPORTED_TYPES:
        return f"invalid type: {move_obj.get('type')}"
    if move_obj.get("category") not in SUPPORTED_CATEGORY:
        return f"invalid category: {move_obj.get('category')}"
    if not isinstance(move_obj.get("steps"), list):
        return "steps must be array"
    return None


def build_name_map(details: list[dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    name_map: dict[str, list[dict[str, Any]]] = {}
    for detail in details:
        for nm in japanese_names(detail):
            key = normalize_name(nm)
            name_map.setdefault(key, []).append(detail)
    return name_map


def resolve_alias_keys(name: str) -> list[str]:
    key = normalize_name(name)
    keys = [key]
    for src, dst in NAME_ALIASES_JA.items():
        if key == normalize_name(src):
            keys.append(normalize_name(dst))
    typo_fix = key.replace("めがい", "ねがい")
    if typo_fix != key:
        keys.append(typo_fix)
    return list(dict.fromkeys(keys))


def collect_details_from_cache(cache_dir: Path) -> list[dict[str, Any]]:
    details: list[dict[str, Any]] = []
    if not cache_dir.exists():
        return details
    for p in sorted(cache_dir.glob("*.json")):
        try:
            details.append(json.loads(p.read_text(encoding="utf-8")))
        except Exception:
            continue
    return details


def ordered_move(move_obj: dict[str, Any]) -> dict[str, Any]:
    keys = ["id", "name", "type", "category", "pp", "power", "accuracy", "priority", "description", "steps", "tags", "critRate"]
    out: dict[str, Any] = {}
    for k in keys:
        if k in move_obj:
            out[k] = move_obj[k]
    for k, v in move_obj.items():
        if k not in out:
            out[k] = v
    return out


def main() -> int:
    args = parse_args()
    repo_root, engine_root = repo_roots()
    csv_path = resolve_path(engine_root, args.csv)
    output_path = resolve_path(engine_root, args.output)
    report_path = resolve_path(engine_root, args.report)
    cache_dir = resolve_path(engine_root, args.cache_dir)

    records = read_csv_records(csv_path)
    if args.max_moves > 0:
        records = records[: args.max_moves]

    details = collect_details_from_cache(cache_dir)
    if not details and not args.strict_cache:
        items = fetch_move_list(args.delay_ms)
        for item in items:
            detail = load_or_fetch_move_detail(cache_dir, item, strict_cache=False, delay_ms=args.delay_ms)
            if detail:
                details.append(detail)
    elif details:
        if not args.strict_cache:
            pass
        else:
            pass

    if not details:
        raise RuntimeError(f"No move detail data found in cache: {cache_dir}")

    name_map = build_name_map(details)
    moves_yaml: dict[str, dict[str, Any]] = {}
    report: dict[str, Any] = {
        "not_found_in_pokeapi": [],
        "ambiguous_matches": [],
        "manual_effects": [],
        "gemini_failed": [],
        "gemini_skipped": [],
        "gemini_updated": 0,
    }

    use_gemini = not args.no_gemini
    api_key = os.getenv("GEMINI_API_KEY", "").strip()
    if use_gemini and not api_key:
        use_gemini = False
        report["gemini_skipped"].append(
            {
                "scope": "global",
                "category": "config",
                "reason": "GEMINI_API_KEY is not set; skipped Gemini refinement",
                "fallback": "base_dsl",
            }
        )

    for rec in records:
        candidates: list[dict[str, Any]] = []
        for key in resolve_alias_keys(rec["name"]):
            candidates = name_map.get(key, [])
            if candidates:
                break
        if not candidates:
            report["not_found_in_pokeapi"].append(rec["name"])
            continue
        if len(candidates) > 1:
            report["ambiguous_matches"].append(
                {
                    "csv_name": rec["name"],
                    "candidate_ids": [to_snake_id(c.get("name", "")) for c in candidates],
                }
            )
        detail = candidates[0]

        move_obj, manual_reasons = build_base_move(rec, detail)
        if manual_reasons:
            report["manual_effects"].append(
                {"move_id": move_obj["id"], "move_name": move_obj["name"], "reasons": manual_reasons}
            )

        if use_gemini:
            refined, err = gemini_refine(
                move_obj,
                rec,
                detail,
                api_key,
                args.model,
                retries=args.gemini_retries,
                timeout_sec=args.gemini_timeout_sec,
            )
            if err:
                report["gemini_failed"].append(
                    {
                        "move_id": move_obj["id"],
                        "category": err.get("category"),
                        "attempts": err.get("attempt"),
                        "reason": err.get("reason"),
                        "fallback": "base_dsl",
                    }
                )
            else:
                if refined.get("steps") != move_obj.get("steps"):
                    report["gemini_updated"] += 1
                move_obj = refined
            if args.gemini_delay_ms > 0:
                time.sleep(args.gemini_delay_ms / 1000.0)

        moves_yaml[move_obj["id"]] = ordered_move(move_obj)

    if not args.dry_run:
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_text(
            yaml.safe_dump(moves_yaml, allow_unicode=True, sort_keys=False, width=120),
            encoding="utf-8",
        )
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")

    print(f"moves: {len(moves_yaml)}")
    print(f"output: {output_path}")
    print(f"report: {report_path}")
    print(f"gemini_used: {use_gemini}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
