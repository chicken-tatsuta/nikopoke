import React, { useMemo, useState } from "react";
import {
  Autocomplete,
  Box,
  ButtonBase,
  Chip,
  Collapse,
  Grid,
  Paper,
  Stack,
  TextField,
  Typography,
} from "@mui/material";
import type { PokemonType } from "../lib/damage";

export type MovePickerMove = {
  id: string;
  name: string;
  type: PokemonType;
  category: string;
  power: number | null;
};

export const TYPE_LABELS: Record<PokemonType, string> = {
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

export const TYPE_CHIP_SX: Record<PokemonType, { bgcolor: string; color: string }> = {
  normal: { bgcolor: "#eeeeee", color: "#424242" },
  fire: { bgcolor: "#ffccbc", color: "#bf360c" },
  water: { bgcolor: "#bbdefb", color: "#0d47a1" },
  electric: { bgcolor: "#fff9c4", color: "#f57f17" },
  grass: { bgcolor: "#c8e6c9", color: "#1b5e20" },
  ice: { bgcolor: "#e0f7fa", color: "#006064" },
  fighting: { bgcolor: "#ffe0b2", color: "#e64a19" },
  poison: { bgcolor: "#e1bee7", color: "#6a1b9a" },
  ground: { bgcolor: "#f0d0a0", color: "#6d4c41" },
  flying: { bgcolor: "#e3f2fd", color: "#1565c0" },
  psychic: { bgcolor: "#f8bbd0", color: "#ad1457" },
  bug: { bgcolor: "#dcedc8", color: "#33691e" },
  rock: { bgcolor: "#f0e0a0", color: "#5d4037" },
  ghost: { bgcolor: "#d1c4e9", color: "#311b92" },
  dragon: { bgcolor: "#c5cae9", color: "#283593" },
  dark: { bgcolor: "#cfd8dc", color: "#263238" },
  steel: { bgcolor: "#eceff1", color: "#37474f" },
  fairy: { bgcolor: "#fce4ec", color: "#ad1457" },
};

const getCategoryLabel = (category: string) => {
  if (category === "physical") return "物理";
  if (category === "special") return "特殊";
  return "変化";
};

const getMoveMetaText = (move: MovePickerMove) => {
  const categoryLabel = getCategoryLabel(move.category);
  return move.category === "status" ? categoryLabel : `${categoryLabel} / 威力${move.power ?? 0}`;
};

const sortMoves = (moves: MovePickerMove[]) => {
  return [...moves].sort((a, b) => {
    if (a.type !== b.type) {
      return TYPE_LABELS[a.type].localeCompare(TYPE_LABELS[b.type], "ja");
    }
    if (a.category !== b.category) {
      return a.category.localeCompare(b.category);
    }
    return (b.power ?? 0) - (a.power ?? 0);
  });
};

type SingleProps = {
  multiple?: false;
  options: MovePickerMove[];
  value: MovePickerMove | null;
  onChange: (value: MovePickerMove | null) => void;
  label: string;
  helperText?: string;
};

type MultiProps = {
  multiple: true;
  options: MovePickerMove[];
  value: MovePickerMove[];
  onChange: (value: MovePickerMove[]) => void;
  label: string;
  helperText?: string;
};

type Props = SingleProps | MultiProps;

const MovePicker: React.FC<Props> = (props) => {
  const [moveTypeFilter, setMoveTypeFilter] = useState<PokemonType | "all">("all");
  const [activeSlot, setActiveSlot] = useState(0);
  const [open, setOpen] = useState(false);
  const sortedMoves = useMemo(() => sortMoves(props.options), [props.options]);
  const availableMoveTypes = useMemo(
    () => Array.from(new Set(sortedMoves.map((move) => move.type))),
    [sortedMoves]
  );
  const displayedMoves = useMemo(() => {
    if (moveTypeFilter === "all") return sortedMoves;
    return sortedMoves.filter((move) => move.type === moveTypeFilter);
  }, [sortedMoves, moveTypeFilter]);

  const typeCounts = useMemo(() => {
    return sortedMoves.reduce<Record<PokemonType, number>>((acc, move) => {
      acc[move.type] = (acc[move.type] ?? 0) + 1;
      return acc;
    }, {} as Record<PokemonType, number>);
  }, [sortedMoves]);

  const handleMultipleSelect = (move: MovePickerMove) => {
    if (!props.multiple) return;

    const nextMoves = [...props.value];
    const existingIndex = nextMoves.findIndex((currentMove) => currentMove.id === move.id);

    if (existingIndex >= 0) {
      setActiveSlot(existingIndex);
      return;
    }

    if (activeSlot < nextMoves.length) {
      nextMoves[activeSlot] = move;
    } else if (nextMoves.length < 4) {
      nextMoves.push(move);
    } else {
      nextMoves[activeSlot] = move;
    }

    props.onChange(nextMoves.slice(0, 4));
    setOpen(false);

    if (activeSlot < 3) {
      setActiveSlot(Math.min(activeSlot + 1, 3));
    }
  };

  const clearSlot = (index: number) => {
    if (!props.multiple) return;
    const nextMoves = props.value.filter((_, moveIndex) => moveIndex !== index);
    props.onChange(nextMoves);
    setActiveSlot(Math.min(index, Math.max(nextMoves.length - 1, 0)));
  };

  return (
    <>
      {props.multiple ? (
        <Stack spacing={1.5}>
          <Box>
            <Typography variant="subtitle2" fontWeight={800} sx={{ mb: 0.75 }}>
              {props.label}
            </Typography>
            <Typography variant="body2" color="text.secondary">
              {props.helperText}
            </Typography>
          </Box>

          <Grid container spacing={1}>
            {Array.from({ length: 4 }, (_, index) => {
              const move = props.value[index] ?? null;
              const selected = activeSlot === index;

              return (
                <Grid key={index} size={{ xs: 12, sm: 6 }}>
                  <ButtonBase
                    onClick={() => {
                      setActiveSlot(index);
                      if (!move) {
                        setOpen(true);
                      }
                    }}
                    sx={{
                      width: "100%",
                      textAlign: "left",
                      borderRadius: "6px",
                    }}
                  >
                    <Paper
                      variant="outlined"
                      sx={{
                        width: "100%",
                        px: 1.5,
                        py: 1.25,
                        minHeight: 80,
                        borderRadius: "6px",
                        display: "flex",
                        alignItems: "center",
                        gap: 1.25,
                        borderColor: selected ? "primary.main" : "divider",
                        boxShadow: 'none',
                        bgcolor: selected ? '#f5eee4' : 'background.paper',
                      }}
                    >
                      <Typography variant="h6" fontWeight={900} color="text.secondary">
                        {index + 1}
                      </Typography>
                      {move ? (
                        <>
                          <Chip
                            size="small"
                            label={TYPE_LABELS[move.type]}
                            sx={{ fontWeight: 800, ...TYPE_CHIP_SX[move.type] }}
                          />
                          <Box sx={{ flex: 1, minWidth: 0 }}>
                            <Typography variant="body1" fontWeight={900} noWrap>
                              {move.name}
                            </Typography>
                            <Typography variant="body2" color="text.secondary">
                              {getMoveMetaText(move)}
                            </Typography>
                          </Box>
                          <Chip
                            label="×"
                            size="small"
                            onClick={(event) => {
                              event.stopPropagation();
                              clearSlot(index);
                            }}
                          />
                        </>
                      ) : (
                        <Typography variant="body1" color="text.secondary">
                          未選択
                        </Typography>
                      )}
                    </Paper>
                  </ButtonBase>
                </Grid>
              );
            })}
          </Grid>

          <Collapse in={open}>
            <Stack spacing={1.5}>
              <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
                <Chip
                  label={`すべて (${sortedMoves.length})`}
                  size="small"
                  color={moveTypeFilter === "all" ? "primary" : "default"}
                  variant={moveTypeFilter === "all" ? "filled" : "outlined"}
                  onClick={() => setMoveTypeFilter("all")}
                />
                {availableMoveTypes.map((type) => (
                  <Chip
                    key={type}
                    label={`${TYPE_LABELS[type]} (${typeCounts[type] ?? 0})`}
                    size="small"
                    onClick={() => setMoveTypeFilter(type)}
                    color={moveTypeFilter === type ? "primary" : "default"}
                    variant={moveTypeFilter === type ? "filled" : "outlined"}
                    sx={moveTypeFilter === type ? undefined : TYPE_CHIP_SX[type]}
                  />
                ))}
              </Stack>

              <Grid container spacing={1}>
                {displayedMoves.map((move) => {
                  const isSelected = props.value.some((selectedMove) => selectedMove.id === move.id);

                  return (
                    <Grid key={move.id} size={{ xs: 12, md: 6 }}>
                      <ButtonBase
                        onClick={() => handleMultipleSelect(move)}
                        sx={{ width: "100%", textAlign: "left", borderRadius: 4 }}
                      >
                        <Paper
                          variant="outlined"
                          sx={{
                            width: "100%",
                            px: 1.5,
                            py: 1.25,
                            borderRadius: "6px",
                            borderColor: isSelected ? "primary.main" : "divider",
                            bgcolor: isSelected ? "#f5eee4" : "background.paper",
                          }}
                        >
                          <Stack direction="row" spacing={1} alignItems="center">
                            <Chip
                              size="small"
                              label={TYPE_LABELS[move.type]}
                              sx={{ fontWeight: 800, ...TYPE_CHIP_SX[move.type] }}
                            />
                            <Typography variant="body1" fontWeight={900} sx={{ flex: 1 }}>
                              {move.name}
                            </Typography>
                            {isSelected ? <Chip size="small" color="primary" label="選択中" /> : null}
                          </Stack>
                          <Typography variant="body2" color="text.secondary" sx={{ mt: 0.75 }}>
                            {getMoveMetaText(move)}
                          </Typography>
                        </Paper>
                      </ButtonBase>
                    </Grid>
                  );
                })}
              </Grid>
            </Stack>
          </Collapse>
        </Stack>
      ) : (
        <Autocomplete
          options={displayedMoves}
          value={props.value}
          onChange={(_, value) => props.onChange(value)}
          getOptionLabel={(option) => option.name}
          groupBy={(option) => TYPE_LABELS[option.type]}
          isOptionEqualToValue={(option, value) => option.id === value.id}
          renderOption={(renderProps, option) => (
            <Box component="li" {...renderProps} sx={{ display: "flex", alignItems: "center", gap: 1 }}>
              <Chip size="small" label={TYPE_LABELS[option.type]} sx={{ fontWeight: 800, ...TYPE_CHIP_SX[option.type] }} />
              <Typography variant="body2" sx={{ flex: 1 }}>
                {option.name}
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {getMoveMetaText(option)}
              </Typography>
            </Box>
          )}
          renderGroup={(params) => (
            <li key={params.key}>
              <Box sx={{ px: 1.5, py: 0.75, bgcolor: "#f5eee4", fontSize: 12, fontWeight: 900, color: "primary.main" }}>
                {params.group}
              </Box>
              <ul style={{ padding: 0, margin: 0 }}>{params.children}</ul>
            </li>
          )}
          renderInput={(params) => (
            <TextField {...params} label={props.label} size="small" helperText={props.helperText} />
          )}
        />
      )}
      {!props.multiple ? (
        <Stack direction="row" spacing={1} useFlexGap flexWrap="wrap">
          <Chip
            label={`すべて (${sortedMoves.length})`}
            size="small"
            color={moveTypeFilter === "all" ? "primary" : "default"}
            variant={moveTypeFilter === "all" ? "filled" : "outlined"}
            onClick={() => setMoveTypeFilter("all")}
          />
          {availableMoveTypes.map((type) => (
            <Chip
              key={type}
              label={`${TYPE_LABELS[type]} (${typeCounts[type] ?? 0})`}
              size="small"
              onClick={() => setMoveTypeFilter(type)}
              color={moveTypeFilter === type ? "primary" : "default"}
              variant={moveTypeFilter === type ? "filled" : "outlined"}
              sx={moveTypeFilter === type ? undefined : TYPE_CHIP_SX[type]}
            />
          ))}
        </Stack>
      ) : null}
    </>
  );
};

export default MovePicker;
