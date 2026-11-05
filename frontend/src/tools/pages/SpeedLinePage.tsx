import { speedRoster } from "../data/catalog";
// src/pages/SpeedLinePage.tsx
import React, { useMemo, useState } from "react";
import {
  Box,
  Card,
  CardContent,
  CardHeader,
  Chip,
  Checkbox,
  FormControlLabel,
  MenuItem,
  Stack,
  Slider,
  TextField,
  Typography,
  Table,
  TableHead,
  TableRow,
  TableCell,
  TableBody,
} from "@mui/material";

type Nature = "+" | "=" | "-";
type Mon = {
  name: string;
  baseSpeed: number;
  types: string;
  ability1: string;
  ability2?: string;
};

// 20匹データ（必要ならここだけ更新）
const ROSTER: Mon[] = speedRoster;

const clamp = (n: number, min: number, max: number) => Math.max(min, Math.min(max, n));
const natureMul = (n: Nature) => (n === "+" ? 1.1 : n === "-" ? 0.9 : 1.0);

function calcSpeedRaw(base: number, ev: number, iv: number, nature: Nature, level = 50) {
  const evTerm = Math.floor(clamp(ev, 0, 32));
  const core = Math.floor(((2 * base + clamp(iv, 0, 31)) * level) / 100) + 5 + evTerm;
  return Math.floor(core * natureMul(nature));
}

function applyStage(raw: number, stage: number) {
  const st = clamp(stage, -6, 6);
  if (st === 0) return raw;
  if (st > 0) return Math.floor((raw * (2 + st)) / 2);
  return Math.floor((raw * 2) / (2 + Math.abs(st)));
}

// 乗算は「掛けて切り捨て」を順番に（スカーフ→追い風→2倍特性→かるわざ…）
function applyMultipliers(v: number, multipliers: number[]) {
  let x = v;
  for (const m of multipliers) x = Math.floor(x * m);
  return x;
}

function calcFinalSpeed(params: {
  base: number;
  ev: number;
  iv: number;
  nature: Nature;
  stage: number;
  scarf: boolean;
  tailwind: boolean;
  x2: boolean;
  unburden: boolean;
}) {
  const raw = calcSpeedRaw(params.base, params.ev, params.iv, params.nature);
  const staged = applyStage(raw, params.stage);
  const mults: number[] = [];
  if (params.scarf) mults.push(1.5);
  if (params.tailwind) mults.push(2);
  if (params.x2) mults.push(2);
  if (params.unburden) mults.push(2);
  const final = applyMultipliers(staged, mults);
  return { raw, staged, final };
}

const SpeedLinePage: React.FC = () => {
  // 自分（デフォ：たつた晴れ=2倍想定）
  const [myName, setMyName] = useState("たつた");
  const [myEV, setMyEV] = useState(0);
  const [myIV, setMyIV] = useState(31);
  const [myNature, setMyNature] = useState<Nature>("=");
  const [myStage, setMyStage] = useState(0);
  const [myScarf, setMyScarf] = useState(false);
  const [myTailwind, setMyTailwind] = useState(false);
  const [myX2, setMyX2] = useState(true); // ようりょくそ/すいすい等
  const [myUnburden, setMyUnburden] = useState(false);

  const [q, setQ] = useState("");
  const [showScarfCols, setShowScarfCols] = useState(true);
  const [showX2Cols, setShowX2Cols] = useState(true);

  const myMon = useMemo(() => ROSTER.find((m) => m.name === myName) ?? ROSTER[1], [myName]);
  const my = useMemo(
    () =>
      calcFinalSpeed({
        base: myMon.baseSpeed,
        ev: myEV,
        iv: myIV,
        nature: myNature,
        stage: myStage,
        scarf: myScarf,
        tailwind: myTailwind,
        x2: myX2,
        unburden: myUnburden,
      }),
    [myMon.baseSpeed, myEV, myIV, myNature, myStage, myScarf, myTailwind, myX2, myUnburden]
  );

  const rows = useMemo(() => {
    const filtered = ROSTER.filter((m) => m.name.includes(q.trim()));
    return filtered
      .map((m) => {
        const mu0 = calcFinalSpeed({ base: m.baseSpeed, ev: 0, iv: 31, nature: "=", stage: 0, scarf: false, tailwind: false, x2: false, unburden: false }).final;
        const jun = calcFinalSpeed({ base: m.baseSpeed, ev: 32, iv: 31, nature: "=", stage: 0, scarf: false, tailwind: false, x2: false, unburden: false }).final;
        const sai = calcFinalSpeed({ base: m.baseSpeed, ev: 32, iv: 31, nature: "+", stage: 0, scarf: false, tailwind: false, x2: false, unburden: false }).final;
        const saiScarf = Math.floor(sai * 1.5);
        const saiX2 = Math.floor(sai * 2);
        const saiScarfX2 = Math.floor(sai * 1.5 * 2);

        return { m, mu0, jun, sai, saiScarf, saiX2, saiScarfX2 };
      })
      .sort((a, b) => b.sai - a.sai);
  }, [q]);

  const cellSx = (their: number) => ({
    fontWeight: 900,
    bgcolor: my.final > their ? "rgba(46, 125, 50, 0.10)" : "rgba(211, 47, 47, 0.06)",
    borderRadius: 1,
  });

  return (
    <Box sx={{ pt: 3 }}>
      <Stack spacing={2.5}>
        <Box>
          <Typography variant="h5" fontWeight={900}>
            Sライン早見
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Lv.50固定 / IV・EV・性格・スカーフ・追い風・2倍特性・かるわざを考慮して比較
          </Typography>
        </Box>

        <Card>
          <CardHeader
            title="自分のS計算"
            subheader={`${myMon.name}（S${myMon.baseSpeed}）/ ${myMon.types} / ${myMon.ability1}${myMon.ability2 ? "・" + myMon.ability2 : ""}`}
          />
          <CardContent>
            <Stack direction={{ xs: "column", md: "row" }} spacing={2.5} alignItems="flex-start">
              <Stack spacing={2} sx={{ minWidth: 280, flex: 1 }}>
                <TextField
                  select
                  label="ポケモン"
                  value={myName}
                  onChange={(e) => setMyName(e.target.value)}
                  size="small"
                >
                  {ROSTER.map((m) => (
                    <MenuItem key={m.name} value={m.name}>
                      {m.name}（S{m.baseSpeed}）
                    </MenuItem>
                  ))}
                </TextField>

                <Stack direction="row" spacing={1.5}>
                  <TextField
                    label="IV"
                    type="number"
                    size="small"
                    value={myIV}
                    onChange={(e) => setMyIV(clamp(parseInt(e.target.value || "0", 10), 0, 31))}
                    inputProps={{ min: 0, max: 31 }}
                    sx={{ width: 110 }}
                  />
                  <TextField
                    select
                    label="性格"
                    size="small"
                    value={myNature}
                    onChange={(e) => setMyNature(e.target.value as Nature)}
                    sx={{ width: 140 }}
                  >
                    <MenuItem value="+">↑（最速）</MenuItem>
                    <MenuItem value="=">→（無振）</MenuItem>
                    <MenuItem value="-">↓（下降）</MenuItem>
                  </TextField>
                  <TextField
                    label="Sランク"
                    type="number"
                    size="small"
                    value={myStage}
                    onChange={(e) => setMyStage(clamp(parseInt(e.target.value || "0", 10), -6, 6))}
                    inputProps={{ min: -6, max: 6 }}
                    sx={{ width: 110 }}
                  />
                </Stack>

                <Box>
                  <Typography variant="body2" fontWeight={800} sx={{ mb: 0.5 }}>
                    EV（{myEV}）
                  </Typography>
                  <Slider
                    value={myEV}
                    onChange={(_, v) => setMyEV(v as number)}
                    min={0}
                    max={32}
                    step={1}
                    valueLabelDisplay="auto"
                  />
                </Box>

                <Stack direction="row" spacing={1} flexWrap="wrap">
                  <FormControlLabel control={<Checkbox checked={myScarf} onChange={(e) => setMyScarf(e.target.checked)} />} label="スカーフ" />
                  <FormControlLabel control={<Checkbox checked={myTailwind} onChange={(e) => setMyTailwind(e.target.checked)} />} label="追い風" />
                  <FormControlLabel control={<Checkbox checked={myX2} onChange={(e) => setMyX2(e.target.checked)} />} label="2倍特性" />
                  <FormControlLabel control={<Checkbox checked={myUnburden} onChange={(e) => setMyUnburden(e.target.checked)} />} label="かるわざ" />
                </Stack>
              </Stack>

              <Box sx={{ minWidth: 260 }}>
                <Stack spacing={1.2}>
                  <Chip label={`S実数値（補正前）: ${my.raw}`} sx={{ fontWeight: 900 }} />
                  <Chip label={`S実数値（ランク後）: ${my.staged}`} sx={{ fontWeight: 900 }} />
                  <Chip label={`最終S: ${my.final}`} color="primary" sx={{ fontWeight: 900, fontSize: 16, py: 2 }} />
                </Stack>
                <Typography variant="caption" color="text.secondary" sx={{ display: "block", mt: 1 }}>
                  ※掛け算は「掛けて切り捨て」を順番に適用
                </Typography>
              </Box>
            </Stack>
          </CardContent>
        </Card>

        <Card>
          <CardHeader
            title="相手のSライン一覧（この20匹）"
            subheader="セルが緑なら「あなたの最終S ＞ 相手S」"
          />
          <CardContent>
            <Stack direction={{ xs: "column", md: "row" }} spacing={2} alignItems="center" sx={{ mb: 2 }}>
              <TextField
                label="検索（名前）"
                value={q}
                onChange={(e) => setQ(e.target.value)}
                size="small"
                sx={{ width: 260 }}
              />
              <FormControlLabel control={<Checkbox checked={showScarfCols} onChange={(e) => setShowScarfCols(e.target.checked)} />} label="スカーフ列を表示" />
              <FormControlLabel control={<Checkbox checked={showX2Cols} onChange={(e) => setShowX2Cols(e.target.checked)} />} label="×2列を表示" />
              <Typography variant="body2" color="text.secondary">
                クリックで自分のポケモンにセット
              </Typography>
            </Stack>

            <Box sx={{ overflowX: "auto" }}>
              <Table size="small" sx={{ minWidth: 920 }}>
                <TableHead>
                  <TableRow>
                    <TableCell sx={{ fontWeight: 900 }}>名前</TableCell>
                    <TableCell sx={{ fontWeight: 900 }}>タイプ/特性</TableCell>
                    <TableCell sx={{ fontWeight: 900 }} align="right">
                      無振り
                    </TableCell>
                    <TableCell sx={{ fontWeight: 900 }} align="right">
                      準速
                    </TableCell>
                    <TableCell sx={{ fontWeight: 900 }} align="right">
                      最速
                    </TableCell>
                    {showScarfCols && (
                      <TableCell sx={{ fontWeight: 900 }} align="right">
                        最速スカーフ
                      </TableCell>
                    )}
                    {showX2Cols && (
                      <TableCell sx={{ fontWeight: 900 }} align="right">
                        最速×2
                      </TableCell>
                    )}
                    {showScarfCols && showX2Cols && (
                      <TableCell sx={{ fontWeight: 900 }} align="right">
                        最速スカーフ×2
                      </TableCell>
                    )}
                  </TableRow>
                </TableHead>

                <TableBody>
                  {rows.map((r) => (
                    <TableRow
                      key={r.m.name}
                      hover
                      onClick={() => setMyName(r.m.name)}
                      sx={{ cursor: "pointer" }}
                    >
                      <TableCell sx={{ fontWeight: 900 }}>
                        {r.m.name} <Typography component="span" variant="caption" color="text.secondary">（S{r.m.baseSpeed}）</Typography>
                      </TableCell>
                      <TableCell>
                        <Stack direction="row" spacing={1} flexWrap="wrap">
                          <Chip size="small" label={r.m.types} />
                          {r.m.ability1 && <Chip size="small" label={r.m.ability1} variant="outlined" />}
                          {r.m.ability2 && <Chip size="small" label={r.m.ability2} variant="outlined" />}
                        </Stack>
                      </TableCell>

                      <TableCell align="right" sx={cellSx(r.mu0)}>{r.mu0}</TableCell>
                      <TableCell align="right" sx={cellSx(r.jun)}>{r.jun}</TableCell>
                      <TableCell align="right" sx={cellSx(r.sai)}>{r.sai}</TableCell>

                      {showScarfCols && (
                        <TableCell align="right" sx={cellSx(r.saiScarf)}>{r.saiScarf}</TableCell>
                      )}
                      {showX2Cols && (
                        <TableCell align="right" sx={cellSx(r.saiX2)}>{r.saiX2}</TableCell>
                      )}
                      {showScarfCols && showX2Cols && (
                        <TableCell align="right" sx={cellSx(r.saiScarfX2)}>{r.saiScarfX2}</TableCell>
                      )}
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </Box>
          </CardContent>
        </Card>
      </Stack>
    </Box>
  );
};

export default SpeedLinePage;
