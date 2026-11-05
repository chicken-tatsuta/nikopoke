#!/usr/bin/env node

const fs = require('fs');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const frontendRoot = path.join(repoRoot, 'frontend');
const engineDataDir = path.join(repoRoot, 'engine-rust', 'data');
const frontendDataDir = path.join(frontendRoot, 'public', 'data');
const defaultCsvPath = path.join(engineDataDir, '2期生男子種族値 - 技一覧.csv');

const yaml = require(
  require.resolve('js-yaml', {
    paths: [frontendRoot],
  }),
);

const CSV_TYPE_MAP = {
  ノーマル: 'normal',
  ほのお: 'fire',
  みず: 'water',
  でんき: 'electric',
  くさ: 'grass',
  こおり: 'ice',
  かくとう: 'fighting',
  どく: 'poison',
  じめん: 'ground',
  ひこう: 'flying',
  エスパー: 'psychic',
  むし: 'bug',
  いわ: 'rock',
  ゴースト: 'ghost',
  ドラゴン: 'dragon',
  あく: 'dark',
  はがね: 'steel',
  フェアリー: 'fairy',
};

const CSV_CATEGORY_MAP = {
  物理: 'physical',
  特殊: 'special',
  変化: 'status',
};

function parseArgs(argv) {
  const options = {
    csvSourcePath: null,
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--csv' && index + 1 < argv.length) {
      options.csvSourcePath = path.resolve(argv[index + 1]);
      index += 1;
    }
    if (arg === '--help' || arg === '-h') {
      printHelp();
      process.exit(0);
    }
  }

  return options;
}

function printHelp() {
  console.log(`sync-battle-data

Usage:
  node scripts/sync-battle-data.cjs [--csv "/absolute/path/to/2期生男子種族値 - 技一覧 (1).csv"]

What it does:
  1. Optionally copies the latest move CSV into engine-rust/data
  2. Rebuilds engine-rust/data/moves.json from engine-rust/data/moves/**/*.yaml
  3. Rebuilds learnsets.yaml/json from the CSV's 配布対象 column
  4. Syncs species/moves/learnsets JSON into frontend/public/data
`);
}

function normalizeLabel(value) {
  return String(value ?? '')
    .replace(/[ \t\r\n　]+/g, '')
    .replace(/[０-９]/g, (digit) => String.fromCharCode(digit.charCodeAt(0) - 0xFEE0))
    .trim();
}

function normalizeComparisonText(value) {
  return normalizeLabel(value)
    .replace(/[『』「」・,，。！!？?\-ー]/g, '');
}

function parseCsv(text) {
  const rows = [];
  let row = [];
  let field = '';
  let inQuotes = false;

  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    const next = text[index + 1];

    if (char === '"') {
      if (inQuotes && next === '"') {
        field += '"';
        index += 1;
      } else {
        inQuotes = !inQuotes;
      }
      continue;
    }

    if (char === ',' && !inQuotes) {
      row.push(field);
      field = '';
      continue;
    }

    if ((char === '\n' || char === '\r') && !inQuotes) {
      if (char === '\r' && next === '\n') {
        index += 1;
      }
      row.push(field);
      rows.push(row);
      row = [];
      field = '';
      continue;
    }

    field += char;
  }

  if (field.length > 0 || row.length > 0) {
    row.push(field);
    rows.push(row);
  }

  return rows;
}

function ensureDirectory(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readText(filePath) {
  return fs.readFileSync(filePath, 'utf8');
}

function loadYamlFile(filePath) {
  return yaml.load(readText(filePath));
}

function listYamlFiles(rootDir) {
  const results = [];

  function walk(currentDir) {
    const entries = fs.readdirSync(currentDir, { withFileTypes: true });
    entries.sort((left, right) => left.name.localeCompare(right.name, 'en'));

    for (const entry of entries) {
      const fullPath = path.join(currentDir, entry.name);
      if (entry.isDirectory()) {
        walk(fullPath);
        continue;
      }
      if (entry.isFile() && entry.name.endsWith('.yaml')) {
        results.push(fullPath);
      }
    }
  }

  walk(rootDir);
  return results;
}

function sortObject(value) {
  if (Array.isArray(value)) {
    return value.map(sortObject);
  }
  if (!value || typeof value !== 'object') {
    return value;
  }

  const sorted = {};
  for (const key of Object.keys(value).sort((left, right) => left.localeCompare(right, 'en'))) {
    sorted[key] = sortObject(value[key]);
  }
  return sorted;
}

function loadMovesFromYamlDir() {
  const movesDir = path.join(engineDataDir, 'moves');
  const files = listYamlFiles(movesDir);
  const moveMap = {};
  const moveIdsByName = new Map();

  function registerMove(rawMove, fallbackId) {
    if (!rawMove || typeof rawMove !== 'object') {
      return;
    }

    const move = { ...rawMove };
    const moveId = String(move.id || fallbackId);
    if (!moveId) {
      return;
    }

    move.id = moveId;
    moveMap[moveId] = move;

    const moveName = normalizeLabel(move.name);
    if (!moveName) {
      return;
    }

    const existing = moveIdsByName.get(moveName) ?? [];
    if (!existing.includes(moveId)) {
      existing.push(moveId);
    }
    moveIdsByName.set(moveName, existing);
  }

  for (const filePath of files) {
    const parsed = loadYamlFile(filePath);
    if (!parsed || typeof parsed !== 'object') {
      continue;
    }

    // Support both:
    // 1. one move per file: { id, name, ... }
    // 2. multiple moves per file: { move_id: { ... }, ... }
    if ('id' in parsed && typeof parsed.id === 'string') {
      registerMove(parsed, path.basename(filePath, '.yaml'));
      continue;
    }

    for (const [key, value] of Object.entries(parsed)) {
      registerMove(value, key);
    }
  }

  const duplicateMoveNames = Array.from(moveIdsByName.entries())
    .filter(([, moveIds]) => moveIds.length > 1)
    .map(([moveName, moveIds]) => ({ moveName, moveIds }));

  return {
    moveMap: sortObject(moveMap),
    moveIdsByName,
    duplicateMoveNames,
  };
}

function loadSpeciesFromJson() {
  const speciesPath = path.join(engineDataDir, 'species.json');
  const speciesMap = JSON.parse(readText(speciesPath));
  const speciesIdByName = new Map();
  const duplicateSpeciesNames = [];

  for (const [speciesId, species] of Object.entries(speciesMap)) {
    if (!species || typeof species !== 'object') {
      continue;
    }

    const speciesName = normalizeLabel(species.name);
    if (!speciesName) {
      continue;
    }

    if (speciesIdByName.has(speciesName) && speciesIdByName.get(speciesName) !== speciesId) {
      duplicateSpeciesNames.push({
        speciesName,
        speciesIds: [speciesIdByName.get(speciesName), speciesId],
      });
      continue;
    }

    speciesIdByName.set(speciesName, speciesId);
  }

  return {
    speciesMap: sortObject(speciesMap),
    speciesIdByName,
    duplicateSpeciesNames,
  };
}

function parseRecipients(rawValue) {
  return String(rawValue ?? '')
    .split(/[、\r\n]+/)
    .map((value) => normalizeLabel(value))
    .filter(Boolean);
}

function parseCsvNumber(rawValue) {
  const normalized = normalizeLabel(rawValue);
  if (!normalized || normalized === '-') {
    return null;
  }
  const number = Number(normalized);
  return Number.isFinite(number) ? number : null;
}

function toCsvAccuracy(accuracyValue) {
  if (accuracyValue === null || accuracyValue === undefined) {
    return null;
  }
  if (accuracyValue <= 1) {
    return Math.round(accuracyValue * 100);
  }
  return Math.round(accuracyValue);
}

function buildBigrams(text) {
  const normalized = normalizeComparisonText(text);
  if (normalized.length < 2) {
    return new Set(normalized ? [normalized] : []);
  }

  const result = new Set();
  for (let index = 0; index < normalized.length - 1; index += 1) {
    result.add(normalized.slice(index, index + 2));
  }
  return result;
}

function scoreDescriptionSimilarity(left, right) {
  const leftBigrams = buildBigrams(left);
  const rightBigrams = buildBigrams(right);
  if (leftBigrams.size === 0 || rightBigrams.size === 0) {
    return 0;
  }

  let shared = 0;
  for (const token of leftBigrams) {
    if (rightBigrams.has(token)) {
      shared += 1;
    }
  }

  return shared / Math.max(leftBigrams.size, rightBigrams.size);
}

function scoreMoveCandidate(move, row) {
  let score = 0;
  const csvType = CSV_TYPE_MAP[String(row[2] ?? '').trim()] ?? null;
  const csvPower = parseCsvNumber(row[3]);
  const csvAccuracy = parseCsvNumber(row[4]);
  const csvPp = parseCsvNumber(row[5]);
  const csvCategory = CSV_CATEGORY_MAP[String(row[6] ?? '').trim()] ?? null;
  const csvContact = String(row[7] ?? '').includes('接触');
  const csvEffect = String(row[8] ?? '');

  if (csvType && move.type === csvType) {
    score += 30;
  }
  if (csvCategory && move.category === csvCategory) {
    score += 20;
  }

  if (csvPower === null) {
    if (move.power === null || move.power === undefined || move.power === 0) {
      score += 8;
    }
  } else if (Number(move.power) === csvPower) {
    score += 12;
  }

  if (csvAccuracy === null) {
    if (move.accuracy === null || move.accuracy === undefined) {
      score += 6;
    }
  } else if (toCsvAccuracy(move.accuracy) === csvAccuracy) {
    score += 10;
  }

  if (csvPp !== null && Number(move.pp) === csvPp) {
    score += 10;
  }

  const moveTags = Array.isArray(move.tags) ? move.tags : [];
  if (csvContact === moveTags.includes('contact')) {
    score += 6;
  }

  score += Math.round(scoreDescriptionSimilarity(csvEffect, move.description) * 40);
  return score;
}

function selectMoveIdForCsvRow(row, moveMap, moveIdsByName) {
  const moveName = normalizeLabel(row[0]);
  const namedCandidates = moveIdsByName.get(moveName) ?? [];

  if (namedCandidates.length === 1) {
    return {
      moveId: namedCandidates[0],
      ambiguousCandidates: [],
    };
  }

  const candidateIds = namedCandidates.length > 0 ? namedCandidates : Object.keys(moveMap);
  const scoredCandidates = candidateIds
    .map((moveId) => ({
      moveId,
      score: scoreMoveCandidate(moveMap[moveId], row),
    }))
    .sort((left, right) => right.score - left.score || left.moveId.localeCompare(right.moveId, 'en'));

  const bestCandidate = scoredCandidates[0];
  if (!bestCandidate || (bestCandidate.score <= 0 && namedCandidates.length === 0)) {
    return {
      moveId: null,
      ambiguousCandidates: [],
    };
  }

  const ambiguousCandidates = scoredCandidates
    .filter((candidate) => candidate.score === bestCandidate.score)
    .map((candidate) => candidate.moveId);

  return {
    moveId: bestCandidate.moveId,
    ambiguousCandidates,
  };
}

function loadMoveIdMigrationMap() {
  const migrationPath = path.join(engineDataDir, 'move_id_migration_report.json');
  const oldToNewIdMap = new Map();
  const migrationRows = [];

  if (!fs.existsSync(migrationPath)) {
    return { oldToNewIdMap, migrationRows };
  }

  const report = JSON.parse(readText(migrationPath));

  function walk(value) {
    if (!value) {
      return;
    }
    if (Array.isArray(value)) {
      for (const entry of value) {
        walk(entry);
      }
      return;
    }
    if (typeof value !== 'object') {
      return;
    }
    if (value.old_id && value.new_id) {
      const oldId = String(value.old_id);
      const newId = String(value.new_id);
      oldToNewIdMap.set(oldId, {
        newId,
        name: value.name ? normalizeLabel(value.name) : null,
      });
      migrationRows.push({ oldId, newId, name: value.name ? String(value.name) : null });
    }
    for (const child of Object.values(value)) {
      walk(child);
    }
  }

  walk(report);
  return { oldToNewIdMap, migrationRows };
}

function buildLearnsetsFromCsv(csvPath, speciesIdByName, moveMap, moveIdsByName, oldToNewIdMap) {
  const rows = parseCsv(readText(csvPath));
  const [, ...dataRows] = rows;
  const learnsets = {};
  const missingMoveNames = new Set();
  const missingSpeciesNames = new Set();
  const ambiguousMatches = [];
  let matchedRows = 0;

  for (const row of dataRows) {
    const moveName = normalizeLabel(row[0]);
    if (!moveName) {
      continue;
    }

    const { moveId, ambiguousCandidates } = selectMoveIdForCsvRow(row, moveMap, moveIdsByName);
    if (!moveId) {
      missingMoveNames.add(moveName);
      continue;
    }
    const migration = oldToNewIdMap.get(moveId);
    const canonicalMoveId = migration && (!migration.name || migration.name === moveName)
      ? migration.newId
      : moveId;

    if (ambiguousCandidates.length > 1) {
      ambiguousMatches.push({
        moveName,
        selectedMoveId: canonicalMoveId,
        candidateMoveIds: ambiguousCandidates,
      });
    }

    matchedRows += 1;

    for (const recipientName of parseRecipients(row[1])) {
      const speciesId = speciesIdByName.get(recipientName);
      if (!speciesId) {
        missingSpeciesNames.add(recipientName);
        continue;
      }

      if (!learnsets[speciesId]) {
        learnsets[speciesId] = [];
      }

      if (!learnsets[speciesId].includes(canonicalMoveId)) {
        learnsets[speciesId].push(canonicalMoveId);
      }
    }
  }

  for (const speciesId of Array.from(speciesIdByName.values()).sort((left, right) => left.localeCompare(right, 'en'))) {
    if (!learnsets[speciesId]) {
      learnsets[speciesId] = [];
    }
  }

  return {
    learnsets: sortObject(learnsets),
    report: {
      totalRows: dataRows.filter((row) => normalizeLabel(row[0])).length,
      matchedRows,
      missingMoveNames: Array.from(missingMoveNames).sort((left, right) => left.localeCompare(right, 'ja')),
      missingSpeciesNames: Array.from(missingSpeciesNames).sort((left, right) => left.localeCompare(right, 'ja')),
      ambiguousMatches,
    },
  };
}

function writeJson(filePath, data) {
  fs.writeFileSync(filePath, `${JSON.stringify(data, null, 2)}\n`);
}

function writeYaml(filePath, data) {
  fs.writeFileSync(
    filePath,
    yaml.dump(data, {
      noRefs: true,
      sortKeys: true,
      lineWidth: 120,
    }),
  );
}

function syncJsonCopies(fileName, data) {
  writeJson(path.join(engineDataDir, fileName), data);
  writeJson(path.join(frontendDataDir, fileName), data);
}

function syncRawJsonCopy(fileName) {
  const sourcePath = path.join(engineDataDir, fileName);
  const destinationPath = path.join(frontendDataDir, fileName);
  if (!fs.existsSync(sourcePath)) {
    return;
  }
  const json = JSON.parse(readText(sourcePath));
  writeJson(destinationPath, json);
}

function main() {
  const options = parseArgs(process.argv.slice(2));
  ensureDirectory(frontendDataDir);

  let csvSourcePath = defaultCsvPath;
  if (options.csvSourcePath) {
    fs.copyFileSync(options.csvSourcePath, defaultCsvPath);
    csvSourcePath = options.csvSourcePath;
  }

  const { moveMap, moveIdsByName, duplicateMoveNames } = loadMovesFromYamlDir();
  const { oldToNewIdMap, migrationRows } = loadMoveIdMigrationMap();
  const { speciesMap, speciesIdByName, duplicateSpeciesNames } = loadSpeciesFromJson();
  const { learnsets, report: learnsetReport } = buildLearnsetsFromCsv(
    defaultCsvPath,
    speciesIdByName,
    moveMap,
    moveIdsByName,
    oldToNewIdMap,
  );

  writeYaml(path.join(engineDataDir, 'moves.yaml'), moveMap);
  syncJsonCopies('moves.json', moveMap);
  syncJsonCopies('species.json', speciesMap);
  writeYaml(path.join(engineDataDir, 'species.yaml'), speciesMap);
  syncJsonCopies('learnsets.json', learnsets);
  syncRawJsonCopy('move_id_migration_report.json');
  syncRawJsonCopy('items.json');
  writeYaml(path.join(engineDataDir, 'learnsets.yaml'), learnsets);

  const syncReport = {
    csvPath: defaultCsvPath,
    copiedCsvFrom: options.csvSourcePath,
    activeCsvSource: csvSourcePath,
    moveCount: Object.keys(moveMap).length,
    speciesCount: Object.keys(speciesMap).length,
    learnsetSpeciesCount: Object.keys(learnsets).length,
    duplicateMoveNames,
    duplicateSpeciesNames,
    moveIdMigrations: migrationRows.length,
    ...learnsetReport,
  };
  writeJson(path.join(engineDataDir, 'sync_report.json'), syncReport);

  console.log(`Synced battle data from ${defaultCsvPath}`);
  console.log(`- moves: ${syncReport.moveCount}`);
  console.log(`- species: ${syncReport.speciesCount}`);
  console.log(`- learnset rows matched: ${syncReport.matchedRows}/${syncReport.totalRows}`);
  console.log(`- missing moves: ${syncReport.missingMoveNames.length}`);
  console.log(`- missing species: ${syncReport.missingSpeciesNames.length}`);
  console.log(`- report: ${path.join(engineDataDir, 'sync_report.json')}`);
}

main();
