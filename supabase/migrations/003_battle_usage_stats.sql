create table if not exists public.battle_stat_events (
  id text primary key,
  mode text not null check (mode in ('ai', 'player')),
  winner_side text not null check (winner_side in ('player', 'opponent')),
  player_user_id uuid references auth.users(id) on delete set null,
  opponent_user_id uuid references auth.users(id) on delete set null,
  player_team jsonb not null,
  opponent_team jsonb not null,
  created_at timestamptz not null default now()
);

alter table public.battle_stat_events
  add column if not exists player_user_id uuid references auth.users(id) on delete set null,
  add column if not exists opponent_user_id uuid references auth.users(id) on delete set null;

create table if not exists public.pokemon_usage_stats (
  species_id text primary key,
  used_count integer not null default 0,
  win_count integer not null default 0,
  loss_count integer not null default 0,
  updated_at timestamptz not null default now()
);

create table if not exists public.pokemon_move_usage_stats (
  species_id text not null,
  move_id text not null,
  used_count integer not null default 0,
  updated_at timestamptz not null default now(),
  primary key (species_id, move_id)
);

create index if not exists idx_pokemon_usage_stats_used_count
  on public.pokemon_usage_stats (used_count desc);

create index if not exists idx_pokemon_move_usage_stats_species_id
  on public.pokemon_move_usage_stats (species_id);

alter table public.battle_stat_events enable row level security;
alter table public.pokemon_usage_stats enable row level security;
alter table public.pokemon_move_usage_stats enable row level security;

drop policy if exists "Anyone can read battle stat events" on public.battle_stat_events;
create policy "Anyone can read battle stat events"
  on public.battle_stat_events
  for select
  to anon, authenticated
  using (true);

drop policy if exists "Anyone can read pokemon usage stats" on public.pokemon_usage_stats;
create policy "Anyone can read pokemon usage stats"
  on public.pokemon_usage_stats
  for select
  to anon, authenticated
  using (true);

drop policy if exists "Anyone can read pokemon move usage stats" on public.pokemon_move_usage_stats;
create policy "Anyone can read pokemon move usage stats"
  on public.pokemon_move_usage_stats
  for select
  to anon, authenticated
  using (true);

create or replace function public.record_battle_result(
  p_battle_id text,
  p_mode text,
  p_winner_side text,
  p_player_team jsonb,
  p_opponent_team jsonb,
  p_player_user_id uuid default null,
  p_opponent_user_id uuid default null
)
returns void
language plpgsql
security definer
set search_path = public
as $$
declare
  inserted_event_id text;
begin
  insert into public.battle_stat_events (
    id,
    mode,
    winner_side,
    player_user_id,
    opponent_user_id,
    player_team,
    opponent_team
  )
  values (
    p_battle_id,
    p_mode,
    p_winner_side,
    p_player_user_id,
    p_opponent_user_id,
    p_player_team,
    p_opponent_team
  )
  on conflict (id) do nothing
  returning id into inserted_event_id;

  if inserted_event_id is null then
    return;
  end if;

  with team_rows as (
    select
      'player'::text as side,
      pokemon ->> 'speciesId' as species_id,
      coalesce(pokemon -> 'moves', '[]'::jsonb) as moves
    from jsonb_array_elements(coalesce(p_player_team, '[]'::jsonb)) as pokemon
    union all
    select
      'opponent'::text as side,
      pokemon ->> 'speciesId' as species_id,
      coalesce(pokemon -> 'moves', '[]'::jsonb) as moves
    from jsonb_array_elements(coalesce(p_opponent_team, '[]'::jsonb)) as pokemon
  ),
  distinct_species as (
    select distinct on (side, species_id)
      side,
      species_id,
      moves
    from team_rows
    where species_id is not null and species_id <> ''
  ),
  species_counts as (
    select
      species_id,
      count(*)::integer as used_count,
      count(*) filter (where side = p_winner_side)::integer as win_count,
      count(*) filter (where side <> p_winner_side)::integer as loss_count
    from distinct_species
    group by species_id
  )
  insert into public.pokemon_usage_stats (species_id, used_count, win_count, loss_count, updated_at)
  select species_id, used_count, win_count, loss_count, now()
  from species_counts
  on conflict (species_id) do update
  set
    used_count = public.pokemon_usage_stats.used_count + excluded.used_count,
    win_count = public.pokemon_usage_stats.win_count + excluded.win_count,
    loss_count = public.pokemon_usage_stats.loss_count + excluded.loss_count,
    updated_at = now();

  with team_rows as (
    select
      'player'::text as side,
      pokemon ->> 'speciesId' as species_id,
      coalesce(pokemon -> 'moves', '[]'::jsonb) as moves
    from jsonb_array_elements(coalesce(p_player_team, '[]'::jsonb)) as pokemon
    union all
    select
      'opponent'::text as side,
      pokemon ->> 'speciesId' as species_id,
      coalesce(pokemon -> 'moves', '[]'::jsonb) as moves
    from jsonb_array_elements(coalesce(p_opponent_team, '[]'::jsonb)) as pokemon
  ),
  distinct_species as (
    select distinct on (side, species_id)
      side,
      species_id,
      moves
    from team_rows
    where species_id is not null and species_id <> ''
  ),
  distinct_moves as (
    select distinct
      species_id,
      move_id
    from distinct_species
    cross join lateral jsonb_array_elements_text(moves) as move_rows(move_id)
    where move_id is not null and move_id <> ''
  ),
  move_counts as (
    select
      species_id,
      move_id,
      count(*)::integer as used_count
    from distinct_moves
    group by species_id, move_id
  )
  insert into public.pokemon_move_usage_stats (species_id, move_id, used_count, updated_at)
  select species_id, move_id, used_count, now()
  from move_counts
  on conflict (species_id, move_id) do update
  set
    used_count = public.pokemon_move_usage_stats.used_count + excluded.used_count,
    updated_at = now();

  if p_winner_side = 'player' then
    if p_player_user_id is not null then
      update public.profiles
      set win_count = win_count + 1
      where id = p_player_user_id;
    end if;

    if p_opponent_user_id is not null then
      update public.profiles
      set loss_count = loss_count + 1
      where id = p_opponent_user_id;
    end if;
  elsif p_winner_side = 'opponent' then
    if p_opponent_user_id is not null then
      update public.profiles
      set win_count = win_count + 1
      where id = p_opponent_user_id;
    end if;

    if p_player_user_id is not null then
      update public.profiles
      set loss_count = loss_count + 1
      where id = p_player_user_id;
    end if;
  end if;
end;
$$;

grant select on table public.battle_stat_events to anon, authenticated;
grant select on table public.pokemon_usage_stats to anon, authenticated;
grant select on table public.pokemon_move_usage_stats to anon, authenticated;
grant execute on function public.record_battle_result(text, text, text, jsonb, jsonb, uuid, uuid) to anon, authenticated;
