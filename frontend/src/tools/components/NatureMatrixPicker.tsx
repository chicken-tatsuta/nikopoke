import React, { useState } from "react";
import { Box, Button, ButtonBase, Collapse, MenuItem, Paper, TextField, Typography } from "@mui/material";
import type { Nature } from "../lib/stats";

export type NatureStatKey = "atk" | "def" | "spa" | "spd" | "spe";

export const NATURE_ROWS: NatureStatKey[] = ["atk", "def", "spa", "spd", "spe"];

export const NATURE_LABELS: Record<NatureStatKey, string> = {
  atk: "こうげき",
  def: "ぼうぎょ",
  spa: "とくこう",
  spd: "とくぼう",
  spe: "すばやさ",
};

export const NATURE_MATRIX: Record<NatureStatKey, Record<NatureStatKey, Nature>> = {
  atk: {
    atk: { name: "がんばりや" },
    def: { name: "さみしがり", increased: "atk", decreased: "def" },
    spa: { name: "いじっぱり", increased: "atk", decreased: "spa" },
    spd: { name: "やんちゃ", increased: "atk", decreased: "spd" },
    spe: { name: "ゆうかん", increased: "atk", decreased: "spe" },
  },
  def: {
    atk: { name: "ずぶとい", increased: "def", decreased: "atk" },
    def: { name: "すなお" },
    spa: { name: "わんぱく", increased: "def", decreased: "spa" },
    spd: { name: "のうてんき", increased: "def", decreased: "spd" },
    spe: { name: "のんき", increased: "def", decreased: "spe" },
  },
  spa: {
    atk: { name: "ひかえめ", increased: "spa", decreased: "atk" },
    def: { name: "おっとり", increased: "spa", decreased: "def" },
    spa: { name: "てれや" },
    spd: { name: "うっかりや", increased: "spa", decreased: "spd" },
    spe: { name: "れいせい", increased: "spa", decreased: "spe" },
  },
  spd: {
    atk: { name: "おだやか", increased: "spd", decreased: "atk" },
    def: { name: "おとなしい", increased: "spd", decreased: "def" },
    spa: { name: "しんちょう", increased: "spd", decreased: "spa" },
    spd: { name: "きまぐれ" },
    spe: { name: "なまいき", increased: "spd", decreased: "spe" },
  },
  spe: {
    atk: { name: "おくびょう", increased: "spe", decreased: "atk" },
    def: { name: "せっかち", increased: "spe", decreased: "def" },
    spa: { name: "ようき", increased: "spe", decreased: "spa" },
    spd: { name: "むじゃき", increased: "spe", decreased: "spd" },
    spe: { name: "まじめ" },
  },
};

export const getNatureDescription = (nature: Nature) => {
  if (!nature.increased || !nature.decreased) return "補正なし";
  return `${NATURE_LABELS[nature.increased]}↑ / ${NATURE_LABELS[nature.decreased]}↓`;
};

type Props = {
  value: Nature;
  onChange: (nature: Nature) => void;
  label?: string;
};

const NatureMatrixPicker: React.FC<Props> = ({ value, onChange, label = "性格" }) => {
  const [open, setOpen] = useState(false);

  return (
    <>
      <TextField
        select
        label={label}
        size="small"
        value={value.name}
        InputProps={{ readOnly: true }}
        helperText={getNatureDescription(value)}
      >
        <MenuItem value={value.name}>{value.name}</MenuItem>
      </TextField>

      <Button
        variant="outlined"
        size="small"
        onClick={() => setOpen((current) => !current)}
        sx={{ alignSelf: "flex-start" }}
      >
        {open ? "性格表を閉じる" : "性格表を開く"}
      </Button>

      <Collapse in={open}>
        <Paper variant="outlined" sx={{ overflow: "hidden" }}>
          <Box
            sx={{
              display: "grid",
              gridTemplateColumns: "92px repeat(5, minmax(0, 1fr))",
              bgcolor: "#fafafa",
            }}
          >
            <Box sx={{ p: 1.25, borderBottom: "1px solid", borderColor: "divider" }} />
            {NATURE_ROWS.map((decreased) => (
              <Box
                key={decreased}
                sx={{ p: 1.25, borderBottom: "1px solid", borderColor: "divider", textAlign: "center" }}
              >
                <Typography variant="caption" fontWeight={800}>
                  {NATURE_LABELS[decreased]}↓
                </Typography>
              </Box>
            ))}

            {NATURE_ROWS.map((increased) => (
              <Box key={increased} sx={{ display: "contents" }}>
                <Box
                  sx={{
                    p: 1.25,
                    bgcolor: "#fafafa",
                    borderTop: "1px solid",
                    borderColor: "divider",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  <Typography variant="caption" fontWeight={800}>
                    {NATURE_LABELS[increased]}↑
                  </Typography>
                </Box>
                {NATURE_ROWS.map((decreased) => {
                  const nature = NATURE_MATRIX[increased][decreased];
                  const selected = value.name === nature.name;

                  return (
                    <ButtonBase
                      key={`${increased}-${decreased}`}
                      onClick={() => {
                        onChange(nature);
                        setOpen(false);
                      }}
                      sx={{
                        p: 1.25,
                        minHeight: 48,
                        borderTop: "1px solid",
                        borderLeft: "1px solid",
                        borderColor: "divider",
                        bgcolor: selected ? "#f5eee4" : "background.paper",
                        "&:hover": {
                          bgcolor: selected ? "#f5eee4" : "#f5eee4",
                        },
                      }}
                    >
                      <Typography variant="body2" fontWeight={selected ? 900 : 600}>
                        {nature.name}
                      </Typography>
                    </ButtonBase>
                  );
                })}
              </Box>
            ))}
          </Box>
        </Paper>
      </Collapse>
    </>
  );
};

export default NatureMatrixPicker;
