// src/pages/PartyTypePage.tsx
import React, { useMemo, useState } from "react";
import {
  Box,
  Container,
  Typography,
  Card,
  CardHeader,
  CardContent,
  Stack,
  Chip,
  Table,
  TableHead,
  TableRow,
  TableCell,
  TableBody,
} from "@mui/material";
import type { PokemonType } from "../lib/damage";
import { pokemonData } from "../data/catalog";
import { getPokemonPortraitSrc } from '../../lib/pokemonImages';

// ==== 型と定数 =================================

type PokemonData = {
  id: string;
  name: string;
  types: PokemonType[];
};

const POKEMONS = pokemonData as PokemonData[];

const MAX_PARTY_SIZE = 6;

// 表示用タイプ名
const TYPE_LABELS: Record<PokemonType, string> = {
  normal: "ノーマル",
  fire: "ほのお",
  water: "みず",
  electric: "でんき",
  grass: "くさ",
  ice: "こおり",
  fighting: "かくとう",
  poison: "どく",
  ground: "じめん",
  flying: "ひこう",
  psychic: "エスパー",
  bug: "むし",
  rock: "いわ",
  ghost: "ゴースト",
  dragon: "ドラゴン",
  dark: "あく",
  steel: "はがね",
  fairy: "フェアリー",
};

const ALL_TYPES: PokemonType[] = [
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
];

// タイプごとのカラー（パステル寄り）
const TYPE_COLORS: Record<PokemonType, { bg: string; text: string }> = {
    normal:  { bg: "#eeeeee", text: "#424242" },
    fire:    { bg: "#ffccbc", text: "#bf360c" },
    water:   { bg: "#bbdefb", text: "#0d47a1" },
    electric:{ bg: "#fff9c4", text: "#f57f17" },
    grass:   { bg: "#c8e6c9", text: "#1b5e20" },
    ice:     { bg: "#e0f7fa", text: "#006064" },
    fighting:{ bg: "#ffe0b2", text: "#e64a19" },
    poison:  { bg: "#e1bee7", text: "#6a1b9a" },
    ground:  { bg: "#E6B04E", text: "#614B14" },
    flying:  { bg: "#e3f2fd", text: "#1565c0" },
    psychic: { bg: "#f8bbd0", text: "#ad1457" },
    bug:     { bg: "#dcedc8", text: "#33691e" },
    rock:    { bg: "#F0D580", text: "#3e2723" },
    ghost:   { bg: "#d1c4e9", text: "#311b92" },
    dragon:  { bg: "#c5cae9", text: "#283593" },
    dark:    { bg: "#cfd8dc", text: "#263238" },
    steel:   { bg: "#eceff1", text: "#37474f" },
    fairy:   { bg: "#fce4ec", text: "#ad1457" },
  };

  // 列見出し用の1文字ラベル
const TYPE_HEADER_LABELS: Record<PokemonType, string> = {
    normal:  "無", // ノーマル
    fire:    "炎",
    water:   "水",
    electric:"電",
    grass:   "草",
    ice:     "氷",
    fighting:"闘",
    poison:  "毒",
    ground:  "地",
    flying:  "飛",
    psychic: "超",
    bug:     "虫",
    rock:    "岩",
    ghost:   "霊",
    dragon:  "竜",
    dark:    "悪",
    steel:   "鋼",
    fairy:   "妖",
  };

  // Chip 用のスタイルヘルパー
  const getTypeChipSx = (type: PokemonType) => {
    const c = TYPE_COLORS[type];
    return {
      bgcolor: c.bg,
      color: c.text,
      fontWeight: 700,
    } as const;
  };

// 攻撃側 → 防御側 のタイプ相性（ダメ計と同じやつ）
const TYPE_CHART: Record<PokemonType, Partial<Record<PokemonType, number>>> = {
  normal: { rock: 0.5, ghost: 0, steel: 0.5 },
  fire: {
    fire: 0.5,
    water: 0.5,
    grass: 2,
    ice: 2,
    bug: 2,
    rock: 0.5,
    dragon: 0.5,
    steel: 2,
  },
  water: {
    fire: 2,
    water: 0.5,
    grass: 0.5,
    ground: 2,
    rock: 2,
    dragon: 0.5,
  },
  electric: {
    water: 2,
    electric: 0.5,
    grass: 0.5,
    ground: 0,
    flying: 2,
    dragon: 0.5,
  },
  grass: {
    fire: 0.5,
    water: 2,
    grass: 0.5,
    poison: 0.5,
    ground: 2,
    flying: 0.5,
    bug: 0.5,
    rock: 2,
    dragon: 0.5,
    steel: 0.5,
  },
  ice: {
    fire: 0.5,
    water: 0.5,
    grass: 2,
    ice: 0.5,
    ground: 2,
    flying: 2,
    dragon: 2,
    steel: 0.5,
  },
  fighting: {
    normal: 2,
    ice: 2,
    rock: 2,
    dark: 2,
    steel: 2,
    poison: 0.5,
    flying: 0.5,
    psychic: 0.5,
    bug: 0.5,
    fairy: 0.5,
    ghost: 0,
  },
  poison: {
    grass: 2,
    fairy: 2,
    poison: 0.5,
    ground: 0.5,
    rock: 0.5,
    ghost: 0.5,
    steel: 0,
  },
  ground: {
    fire: 2,
    electric: 2,
    poison: 2,
    rock: 2,
    steel: 2,
    grass: 0.5,
    bug: 0.5,
    flying: 0,
  },
  flying: {
    grass: 2,
    fighting: 2,
    bug: 2,
    electric: 0.5,
    rock: 0.5,
    steel: 0.5,
  },
  psychic: {
    fighting: 2,
    poison: 2,
    psychic: 0.5,
    steel: 0.5,
    dark: 0,
  },
  bug: {
    grass: 2,
    psychic: 2,
    dark: 2,
    fire: 0.5,
    fighting: 0.5,
    poison: 0.5,
    flying: 0.5,
    ghost: 0.5,
    steel: 0.5,
    fairy: 0.5,
  },
  rock: {
    fire: 2,
    ice: 2,
    flying: 2,
    bug: 2,
    fighting: 0.5,
    ground: 0.5,
    steel: 0.5,
  },
  ghost: {
    psychic: 2,
    ghost: 2,
    normal: 0,
    dark: 0.5,
  },
  dragon: {
    dragon: 2,
    steel: 0.5,
    fairy: 0,
  },
  dark: {
    psychic: 2,
    ghost: 2,
    fighting: 0.5,
    dark: 0.5,
    fairy: 0.5,
  },
  steel: {
    ice: 2,
    rock: 2,
    fairy: 2,
    fire: 0.5,
    water: 0.5,
    electric: 0.5,
    steel: 0.5,
  },
  fairy: {
    fighting: 2,
    dragon: 2,
    dark: 2,
    fire: 0.5,
    poison: 0.5,
    steel: 0.5,
  },
};

const getTypeMultiplier = (
  moveType: PokemonType,
  defenderTypes: PokemonType[]
): number => {
  const chart = TYPE_CHART[moveType] ?? {};
  return defenderTypes.reduce((mul, t) => mul * (chart[t] ?? 1), 1);
};

type CoverageRow = {
  type: PokemonType;
  weak: number;   // 2倍以上で刺さる味方の数（被ダメ側の弱点）
  resist: number; // 0.5倍以下で受けられる味方の数
  immune: number; // 無効で受けられる味方の数
};

// パーティ全体での「受け相性」を計算
const calcCoverage = (party: PokemonData[]): CoverageRow[] => {
  const base: CoverageRow[] = ALL_TYPES.map((t) => ({
    type: t,
    weak: 0,
    resist: 0,
    immune: 0,
  }));

  const map = new Map<PokemonType, CoverageRow>();
  base.forEach((row) => map.set(row.type, row));

  party.forEach((p) => {
    ALL_TYPES.forEach((atk) => {
      const mult = getTypeMultiplier(atk, p.types);
      const row = map.get(atk);
      if (!row) return;

      if (mult === 0) row.immune += 1;
      else if (mult > 1) row.weak += 1;
      else if (mult < 1) row.resist += 1;
    });
  });

  // 弱点が多いタイプほど上に表示
  return [...base].sort((a, b) => {
    if (b.weak !== a.weak) return b.weak - a.weak;
    // 同じ弱点数なら「耐性少ない方」を上に
    return a.resist - b.resist;
  });
};

// ダメージ倍率を記号＋色に変換（●,○,△,▲,×）
const describeMultiplier = (mult: number) => {
  if (mult === 0) {
    return { symbol: "×", color: "text.disabled", label: "無効" };
  }
  if (mult >= 4) {
    return { symbol: "●", color: "error.main", label: "4倍" };
  }
  if (mult > 1) {
    return { symbol: "○", color: "error.main", label: "2倍" };
  }
  if (mult <= 0.25) {
    return { symbol: "▲", color: "primary.main", label: "0.25倍" };
  }
  if (mult < 1) {
    return { symbol: "△", color: "primary.main", label: "0.5倍" };
  }
  return { symbol: "", color: "text.secondary", label: "等倍" };
};

// 倍率 → スコア変換（高いほどそのタイプに強い）
const scoreFromMultiplier = (mult: number): number => {
    if (mult === 0) return 4;        // 無効
    if (mult <= 0.25) return 3;      // 1/4
    if (mult < 1) return 2;          // 1/2
    if (mult === 1) return 1;        // 等倍
    if (mult >= 4) return -3;        // 4倍
    if (mult > 1) return -1;         // 2倍
    return 0;
  };

  type TypeScore = {
    type: PokemonType;
    score: number;
  };

  type TypeSuggestion = {
    types: PokemonType[];        // 例: ["steel"], ["water","fairy"]
    score: number;               // 改善度（大きいほど◎）
    immuneTypes: PokemonType[];  // 無効にできる一貫タイプ
    resistTypes: PokemonType[];  // 半減以下にできる一貫タイプ
  };

  // サジェスト用：等倍は 0 点（改善しない）、半減以下だけプラス評価
  const suggestionScoreFromMultiplier = (mult: number): number => {
    if (mult === 0) return 4;        // 無効
    if (mult <= 0.25) return 3;      // 1/4
    if (mult < 1) return 2;          // 1/2
    if (mult === 1) return 0;        // 等倍は改善じゃないので 0
    if (mult >= 4) return -3;        // 4倍
    if (mult > 1) return -1;         // 2倍
    return 0;
  };
// ==== コンポーネント本体 =================================

const PartyTypePage: React.FC = () => {
  const [selectedIds, setSelectedIds] = useState<string[]>([]);

  const party = useMemo(
    () => POKEMONS.filter((p) => selectedIds.includes(p.id)),
    [selectedIds]
  );
  const coverage = useMemo(() => calcCoverage(party), [party]);


  const typeScores = useMemo<TypeScore[]>(() => {
    if (party.length === 0) return [];

    return ALL_TYPES.map((atk) => {
      let score = 0;

      party.forEach((p) => {
        const mult = getTypeMultiplier(atk, p.types);
        score += scoreFromMultiplier(mult);
      });

      return { type: atk, score };
    }).sort((a, b) => b.score - a.score); // 強い順に並べる
  }, [party]);

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) => {
      if (prev.includes(id)) {
        return prev.filter((x) => x !== id);
      }
      if (prev.length >= MAX_PARTY_SIZE) {
        // 6匹制限
        return prev;
      }
      return [...prev, id];
    });
  };

  const isSelected = (id: string) => selectedIds.includes(id);

  // 苦手なタイプ（weak>0だけ）
  const dangerousTypes = coverage.filter((row) => row.weak > 0);

  const consistentTypes = coverage.filter(
    (row) => party.length > 0 && row.resist === 0 && row.immune === 0
  );

  const typeSuggestions = useMemo<TypeSuggestion[]>(() => {
    if (party.length === 0) return [];

    // 一貫しているタイプだけをターゲットにする
    const threatTypes = consistentTypes.map((row) => row.type);
    if (threatTypes.length === 0) return [];

    // 候補: 単タイプだけ
    const candidates: PokemonType[][] = ALL_TYPES.map((t) => [t]);

    const scored = candidates
      .map((types) => {
        let score = 0;
        const immuneTypes: PokemonType[] = [];
        const resistTypes: PokemonType[] = [];

        threatTypes.forEach((atk) => {
          const mult = getTypeMultiplier(atk, types);
          const s = suggestionScoreFromMultiplier(mult);
          score += s;

          if (mult === 0) {
            immuneTypes.push(atk);
          } else if (mult < 1) {
            resistTypes.push(atk);
          }
        });

        return { types, score, immuneTypes, resistTypes };
      })
      // 一切改善しない（全部等倍以上）ものは捨てる
      .filter(
        (s) =>
          s.score > 0 &&
          (s.immuneTypes.length > 0 || s.resistTypes.length > 0)
      )
      // スコア高い順
      .sort((a, b) => b.score - a.score)
      // 上位 5 個だけ表示（ここは好みで調整）
      .slice(0, 5);

    return scored;
  }, [party, consistentTypes]);
  return (
    <Box sx={{ pt: 3, pb: 6 }}>
      <Container maxWidth={false} sx={{ px: { xs: 2, sm: 3, md: 4 } }}>
        <Typography variant="h4" fontWeight={700} gutterBottom>
          パーティタイプ補完
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
          2期ポケから最大6匹まで選ぶと、パーティ全体の受け相性が下に出るよ。
        </Typography>

        {/* パーティ選択 */}
        <Card>
          <CardHeader
            title="パーティ選択"
            subheader={`選択中: ${selectedIds.length} / ${MAX_PARTY_SIZE} 匹`}
          />
          <CardContent>
            <Box
              sx={{
                display: "grid",
                gridTemplateColumns: {
                  xs: "repeat(2, minmax(0, 1fr))",
                  sm: "repeat(3, minmax(0, 1fr))",
                  md: "repeat(4, minmax(0, 1fr))",
                },
                gap: 1.5,
              }}
            >
              {POKEMONS.map((p) => {
                const selected = isSelected(p.id);
                return (
                    <Box
                    component="button"
                    type="button"
                    aria-pressed={selected}
                    aria-label={p.name}
                    key={p.id}
                    onClick={() => toggleSelect(p.id)}
                    sx={{
                      p: 1,
                      display: 'grid',
                      gridTemplateColumns: '40px minmax(0, 1fr)',
                      gap: 1,
                      alignItems: 'center',
                      textAlign: 'left',
                      minHeight: 76,
                      borderRadius: '6px',
                      border: "1px solid",
                      borderColor: selected ? '#111111' : '#cccccc',
                      bgcolor: selected ? '#f5eee4' : 'background.paper',
                      cursor: "pointer",
                      transition: "0.15s",
                      "&:hover": {
                        bgcolor: '#f5eee4',
                      },
                      '&:focus-visible': { outline: '2px solid #111111', outlineOffset: 2 },
                    }}
                  >
                    <Box component="img" src={getPokemonPortraitSrc(p.id, p.name)} alt="" loading="lazy"
                      sx={{ width: 40, height: 48, objectFit: 'cover', borderRadius: '4px', border: '1px solid #111111' }} />
                    <Box sx={{ minWidth: 0 }}>
                    <Typography
                      fontWeight={700}
                      fontSize={14}
                      noWrap
                      sx={{ mb: 0.5 }}
                    >
                      {p.name}
                    </Typography>
                    <Stack direction="row" spacing={0.5} useFlexGap flexWrap="wrap">
                    {p.types.map((t) => (
  <Chip
    key={t}
    size="small"
    label={TYPE_LABELS[t]}
    sx={{
      fontSize: "0.7rem",
      ...getTypeChipSx(t),
    }}
  />
))}
                    </Stack>
                    </Box>
                  </Box>
                );
              })}
            </Box>
          </CardContent>
        </Card>

        {/* 受け相性サマリー */}
        <Card sx={{ mt: 3 }}>
          <CardHeader title="受け相性" />
          <CardContent>
          {party.length === 0 ? (
  <Typography variant="body2" color="text.secondary">
    まずは上からパーティメンバーを選んでね。
  </Typography>
) : (
  <>
    {/* 一貫しているタイプ */}
    <Typography
      variant="subtitle2"
      sx={{ fontWeight: 700, mb: 1 }}
    >
      一貫しているタイプ … 全員が等倍以上で受けてしまうタイプ
    </Typography>
    <Box
      sx={{
        mb: 2,
        p: 1.2,
        borderRadius: 0,
        bgcolor: '#fafafa',
      }}
    >
      {consistentTypes.length === 0 ? (
        <Typography variant="body2" color="text.secondary">
          一貫しているタイプはありません（誰かが半減か無効にできます）。
        </Typography>
      ) : (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
          {consistentTypes.map((row) => (
            <Chip
            key={row.type}
            label={
              row.weak > 0
                ? `${TYPE_LABELS[row.type]}（弱点${row.weak}匹）`
                : `${TYPE_LABELS[row.type]}（全員等倍）`
            }
            size="small"
            sx={{
              ...getTypeChipSx(row.type),
              border: row.weak >= 2 ? "2px solid rgba(244,67,54,0.6)" : "none",
            }}
          />
          ))}
        </Stack>
      )}
    </Box>
                {/* 一貫タイプをケアしてくれるタイプ候補 */}
    {typeSuggestions.length > 0 && (
      <Box
        sx={{
          mb: 2,
          p: 1.2,
          borderRadius: 0,
          bgcolor: '#fafafa',
        }}
      >
        <Typography
          variant="subtitle2"
          sx={{ fontWeight: 700, mb: 0.5 }}
        >
          追加すると受け相性が良くなるタイプ候補
        </Typography>

        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ display: "block", mb: 1 }}
        >
          一貫しているタイプに対して、半減以下・無効で受けられる単タイプ／複合タイプを
          スコア順に表示しています（スコアが大きいほど改善度が高い）。
        </Typography>

        <Stack spacing={1.0}>
          {typeSuggestions.map((sug) => (
            <Box
              key={sug.types.join("-")}
              sx={{
                p: 0.75,
                borderRadius: '6px',
                border: "1px solid",
                borderColor: "divider",
                bgcolor: "background.paper",
              }}
            >
              <Stack
                direction="row"
                justifyContent="space-between"
                alignItems="center"
              >
                {/* タイプ組み合わせ表示 */}
                <Stack direction="row" spacing={0.5}>
                  {sug.types.map((t) => (
                    <Chip
                      key={t}
                      size="small"
                      label={TYPE_LABELS[t]}
                      sx={{
                        fontSize: "0.75rem",
                        ...getTypeChipSx(t),
                      }}
                    />
                  ))}
                </Stack>

                <Typography
                  variant="body2"
                  sx={{ fontWeight: 700 }}
                >
                  スコア {sug.score > 0 ? `+${sug.score}` : sug.score}
                </Typography>
              </Stack>

              {(sug.immuneTypes.length > 0 ||
                sug.resistTypes.length > 0) && (
                <Stack
                  direction="row"
                  spacing={1}
                  useFlexGap
                  flexWrap="wrap"
                  sx={{ mt: 0.5 }}
                >
                  {sug.immuneTypes.length > 0 && (
                    <Stack
                      direction="row"
                      spacing={0.5}
                      alignItems="center"
                    >
                      <Typography
                        variant="caption"
                        sx={{ fontWeight: 700 }}
                      >
                        無効:
                      </Typography>
                      {sug.immuneTypes.map((t) => (
                        <Chip
                          key={t}
                          size="small"
                          label={TYPE_HEADER_LABELS[t]}
                          sx={{
                            height: 20,
                            fontSize: "0.7rem",
                            ...getTypeChipSx(t),
                          }}
                        />
                      ))}
                    </Stack>
                  )}

                  {sug.resistTypes.length > 0 && (
                    <Stack
                      direction="row"
                      spacing={0.5}
                      alignItems="center"
                    >
                      <Typography
                        variant="caption"
                        sx={{ fontWeight: 700 }}
                      >
                        半減:
                      </Typography>
                      {sug.resistTypes.map((t) => (
                        <Chip
                          key={t}
                          size="small"
                          label={TYPE_HEADER_LABELS[t]}
                          sx={{
                            height: 20,
                            fontSize: "0.7rem",
                            ...getTypeChipSx(t),
                          }}
                        />
                      ))}
                    </Stack>
                  )}
                </Stack>
              )}
            </Box>
          ))}
        </Stack>
      </Box>
    )}
    {/* 苦手なタイプ（弱点の多さベース・おまけ） */}
    <Typography
      variant="subtitle2"
      sx={{ fontWeight: 700, mb: 1, color: "error.main" }}
    >
      苦手なタイプ … 弱点を突かれるポケモンが多いタイプ
    </Typography>
    <Box
      sx={{
        mb: 2,
        p: 1.2,
        borderRadius: 2,
        bgcolor: "rgba(239,68,68,0.02)",
      }}
    >
      {dangerousTypes.length === 0 ? (
        <Typography variant="body2" color="text.secondary">
          特に弱点が集中しているタイプはありません。
        </Typography>
      ) : (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
          {dangerousTypes.map((row) => (
            <Chip
            key={row.type}
            label={
              row.weak >= 2
                ? `${TYPE_LABELS[row.type]} x${row.weak}`
                : TYPE_LABELS[row.type]
            }
            size="small"
            sx={{
              ...getTypeChipSx(row.type),
              border: row.weak >= 3 ? "2px solid rgba(244,67,54,0.8)" : "none",
            }}
          />
          ))}
        </Stack>
      )}
    </Box>

                {/* 弱点相性表：ポケモン×タイプの行列 */}
                <Typography variant="subtitle2" sx={{ fontWeight: 700, mb: 1 }}>
                  弱点相性表
                </Typography>

                <Table size="small">
                <TableHead>
  <TableRow>
    <TableCell sx={{ width: 140 }} />
    {ALL_TYPES.map((t) => (
      <TableCell
        key={t}
        align="center"
        sx={{ fontSize: 12, p: 0.5 }}
      >
        {TYPE_HEADER_LABELS[t]}
      </TableCell>
    ))}
  </TableRow>
</TableHead>
                  <TableBody>
                    {party.map((p) => (
                      <TableRow key={p.id}>
                        {/* ポケモン名＋タイプChip */}
                        <TableCell
                          sx={{
                            fontSize: 13,
                            borderRight: "1px solid",
                            borderColor: "divider",
                          }}
                        >
                          <Typography fontWeight={700} fontSize={13}>
                            {p.name}
                          </Typography>
                          <Stack
                            direction="row"
                            spacing={0.5}
                            useFlexGap
                            flexWrap="wrap"
                            sx={{ mt: 0.25 }}
                          >
                            {p.types.map((t) => (
  <Chip
    key={t}
    size="small"
    label={TYPE_LABELS[t]}
    sx={{
      fontSize: "0.65rem",
      ...getTypeChipSx(t),
    }}
  />
))}
                          </Stack>
                        </TableCell>

                        {/* 各タイプへの倍率を記号で表示 */}
                        {ALL_TYPES.map((atk) => {
                          const mult = getTypeMultiplier(atk, p.types);
                          const { symbol, color } = describeMultiplier(mult);

                          return (
                            <TableCell
                              key={atk}
                              align="center"
                              sx={{
                                fontSize: 13,
                                p: 0.5,
                                color:
                                  symbol === "" ? "text.disabled" : color,
                                fontWeight: symbol ? 700 : 400,
                              }}
                            >
                              {symbol}
                            </TableCell>
                          );
                        })}
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>

                {/* 凡例 */}
                <Stack
                  direction="row"
                  spacing={2}
                  sx={{ mt: 1, fontSize: 12, color: "text.secondary" }}
                >
                  <Box>
                    <Box component="span" sx={{ color: "error.main" }}>
                      ●
                    </Box>{" "}
                    4倍
                  </Box>
                  <Box>
                    <Box component="span" sx={{ color: "error.main" }}>
                      ○
                    </Box>{" "}
                    2倍
                  </Box>
                  <Box>
                    <Box component="span" sx={{ color: "primary.main" }}>
                      △
                    </Box>{" "}
                    0.5倍
                  </Box>
                  <Box>
                    <Box component="span" sx={{ color: "primary.main" }}>
                      ▲
                    </Box>{" "}
                    0.25倍
                  </Box>
                  <Box>
                    <Box component="span" sx={{ color: "text.disabled" }}>
                      ×
                    </Box>{" "}
                    無効
                  </Box>
                </Stack>
                {party.length > 0 && (
  <>
    <Box sx={{ mt: 2 }}>
      <Typography
        variant="subtitle2"
        sx={{ fontWeight: 700, mb: 1 }}
      >
        タイプ別受けスコア（高いほどそのタイプに強い）
      </Typography>

      <Typography
        variant="caption"
        color="text.secondary"
        sx={{ display: "block", mb: 0.5 }}
      >
        無効:+4 / 1/4:+3 / 1/2:+2 / 等倍:+1 / 2倍:-1 / 4倍:-3 の合計値
      </Typography>

      <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
        {typeScores.map((ts) => (
          <Chip
            key={ts.type}
            size="small"
            label={`${TYPE_HEADER_LABELS[ts.type]} ${ts.score >= 0 ? "+" : ""}${ts.score}`}
            sx={{
              fontSize: "0.8rem",
              ...getTypeChipSx(ts.type),
            }}
          />
        ))}
      </Stack>
    </Box>
  </>
)}
              </>
            )}
          </CardContent>
        </Card>
      </Container>
    </Box>
  );
};

export default PartyTypePage;
