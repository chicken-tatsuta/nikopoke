-- Create moves table
CREATE TABLE moves (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  type TEXT,
  category TEXT,
  pp INTEGER,
  power INTEGER,
  accuracy NUMERIC,
  priority INTEGER DEFAULT 0,
  description TEXT,
  tags TEXT[],
  steps JSONB,
  extra_data JSONB,
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Index for searching
CREATE INDEX idx_moves_name ON moves USING GIN (to_tsvector('simple', name));
CREATE INDEX idx_moves_type ON moves (type);
CREATE INDEX idx_moves_id ON moves (id);

-- Enable Row Level Security (RLS)
ALTER TABLE moves ENABLE ROW LEVEL SECURITY;

-- Create policy to allow anyone to read
CREATE POLICY "Allow public read access" ON moves FOR SELECT USING (true);

-- Create policy to allow authenticated users to update (or just public for now if requested, but better keep it safe)
-- CREATE POLICY "Allow public update access" ON moves FOR UPDATE USING (true);

-- Battle result events used for deduplication and auditability.
CREATE TABLE IF NOT EXISTS battle_stat_events (
  id TEXT PRIMARY KEY,
  mode TEXT NOT NULL CHECK (mode IN ('ai', 'player')),
  winner_side TEXT NOT NULL CHECK (winner_side IN ('player', 'opponent')),
  player_team JSONB NOT NULL,
  opponent_team JSONB NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS pokemon_usage_stats (
  species_id TEXT PRIMARY KEY,
  used_count INTEGER NOT NULL DEFAULT 0,
  win_count INTEGER NOT NULL DEFAULT 0,
  loss_count INTEGER NOT NULL DEFAULT 0,
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS pokemon_move_usage_stats (
  species_id TEXT NOT NULL,
  move_id TEXT NOT NULL,
  used_count INTEGER NOT NULL DEFAULT 0,
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (species_id, move_id)
);

CREATE INDEX IF NOT EXISTS idx_pokemon_usage_stats_used_count ON pokemon_usage_stats (used_count DESC);
CREATE INDEX IF NOT EXISTS idx_pokemon_move_usage_stats_species_id ON pokemon_move_usage_stats (species_id);

ALTER TABLE battle_stat_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE pokemon_usage_stats ENABLE ROW LEVEL SECURITY;
ALTER TABLE pokemon_move_usage_stats ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Allow public read access to battle_stat_events" ON battle_stat_events;
CREATE POLICY "Allow public read access to battle_stat_events" ON battle_stat_events FOR SELECT USING (true);

DROP POLICY IF EXISTS "Allow public read access to pokemon_usage_stats" ON pokemon_usage_stats;
CREATE POLICY "Allow public read access to pokemon_usage_stats" ON pokemon_usage_stats FOR SELECT USING (true);

DROP POLICY IF EXISTS "Allow public read access to pokemon_move_usage_stats" ON pokemon_move_usage_stats;
CREATE POLICY "Allow public read access to pokemon_move_usage_stats" ON pokemon_move_usage_stats FOR SELECT USING (true);

CREATE OR REPLACE FUNCTION public.record_battle_result(
  p_battle_id TEXT,
  p_mode TEXT,
  p_winner_side TEXT,
  p_player_team JSONB,
  p_opponent_team JSONB
)
RETURNS VOID
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  inserted_event_id TEXT;
BEGIN
  INSERT INTO battle_stat_events (id, mode, winner_side, player_team, opponent_team)
  VALUES (p_battle_id, p_mode, p_winner_side, p_player_team, p_opponent_team)
  ON CONFLICT (id) DO NOTHING
  RETURNING id INTO inserted_event_id;

  IF inserted_event_id IS NULL THEN
    RETURN;
  END IF;

  WITH distinct_species AS (
    SELECT DISTINCT ON (side, species_id)
      side,
      species_id,
      moves
    FROM (
      SELECT
        'player'::TEXT AS side,
        pokemon ->> 'speciesId' AS species_id,
        COALESCE(pokemon -> 'moves', '[]'::JSONB) AS moves
      FROM jsonb_array_elements(p_player_team) AS pokemon
      UNION ALL
      SELECT
        'opponent'::TEXT AS side,
        pokemon ->> 'speciesId' AS species_id,
        COALESCE(pokemon -> 'moves', '[]'::JSONB) AS moves
      FROM jsonb_array_elements(p_opponent_team) AS pokemon
    ) team_rows
    WHERE species_id IS NOT NULL AND species_id <> ''
  )
  INSERT INTO pokemon_usage_stats (species_id, used_count, win_count, loss_count, updated_at)
  SELECT
    species_id,
    1,
    CASE WHEN side = p_winner_side THEN 1 ELSE 0 END,
    CASE WHEN side = p_winner_side THEN 0 ELSE 1 END,
    CURRENT_TIMESTAMP
  FROM distinct_species
  ON CONFLICT (species_id) DO UPDATE
  SET
    used_count = pokemon_usage_stats.used_count + EXCLUDED.used_count,
    win_count = pokemon_usage_stats.win_count + EXCLUDED.win_count,
    loss_count = pokemon_usage_stats.loss_count + EXCLUDED.loss_count,
    updated_at = CURRENT_TIMESTAMP;

  WITH distinct_species AS (
    SELECT DISTINCT ON (side, species_id)
      side,
      species_id,
      moves
    FROM (
      SELECT
        'player'::TEXT AS side,
        pokemon ->> 'speciesId' AS species_id,
        COALESCE(pokemon -> 'moves', '[]'::JSONB) AS moves
      FROM jsonb_array_elements(p_player_team) AS pokemon
      UNION ALL
      SELECT
        'opponent'::TEXT AS side,
        pokemon ->> 'speciesId' AS species_id,
        COALESCE(pokemon -> 'moves', '[]'::JSONB) AS moves
      FROM jsonb_array_elements(p_opponent_team) AS pokemon
    ) team_rows
    WHERE species_id IS NOT NULL AND species_id <> ''
  ),
  distinct_moves AS (
    SELECT
      species_id,
      move_id
    FROM distinct_species
    CROSS JOIN LATERAL (
      SELECT DISTINCT jsonb_array_elements_text(moves) AS move_id
    ) move_rows
    WHERE move_id IS NOT NULL AND move_id <> ''
  )
  INSERT INTO pokemon_move_usage_stats (species_id, move_id, used_count, updated_at)
  SELECT species_id, move_id, 1, CURRENT_TIMESTAMP
  FROM distinct_moves
  ON CONFLICT (species_id, move_id) DO UPDATE
  SET
    used_count = pokemon_move_usage_stats.used_count + EXCLUDED.used_count,
    updated_at = CURRENT_TIMESTAMP;
END;
$$;

GRANT EXECUTE ON FUNCTION public.record_battle_result(TEXT, TEXT, TEXT, JSONB, JSONB) TO anon, authenticated;
