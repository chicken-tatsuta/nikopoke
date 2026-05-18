create or replace function public.record_battle_result(
  p_battle_id text,
  p_mode text,
  p_winner_side text,
  p_player_team jsonb,
  p_opponent_team jsonb,
  p_player_user_id uuid default null,
  p_opponent_user_id uuid default null,
  out out_winner_delta integer,
  out out_loser_delta integer,
  out out_winner_elo_delta integer,
  out out_loser_elo_delta integer,
  out out_winner_bonus integer,
  out out_loser_bonus integer
)
language plpgsql
security definer
set search_path = public
as $$
declare
  inserted_event_id text;
  wd integer;
  ld integer;
  we integer;
  le integer;
  wb integer;
  lb integer;
begin
  out_winner_delta := 0;
  out_loser_delta := 0;
  out_winner_elo_delta := 0;
  out_loser_elo_delta := 0;
  out_winner_bonus := 0;
  out_loser_bonus := 0;

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
    select
      rating_result.out_winner_delta,
      rating_result.out_loser_delta,
      rating_result.out_winner_elo_delta,
      rating_result.out_loser_elo_delta,
      rating_result.out_winner_bonus,
      rating_result.out_loser_bonus
    into wd, ld, we, le, wb, lb
    from public.apply_profile_rating_result(p_player_user_id, p_opponent_user_id) as rating_result;
  elsif p_winner_side = 'opponent' then
    select
      rating_result.out_winner_delta,
      rating_result.out_loser_delta,
      rating_result.out_winner_elo_delta,
      rating_result.out_loser_elo_delta,
      rating_result.out_winner_bonus,
      rating_result.out_loser_bonus
    into wd, ld, we, le, wb, lb
    from public.apply_profile_rating_result(p_opponent_user_id, p_player_user_id) as rating_result;
  end if;

  out_winner_delta := coalesce(wd, 0);
  out_loser_delta := coalesce(ld, 0);
  out_winner_elo_delta := coalesce(we, 0);
  out_loser_elo_delta := coalesce(le, 0);
  out_winner_bonus := coalesce(wb, 0);
  out_loser_bonus := coalesce(lb, 0);
end;
$$;

grant execute on function public.record_battle_result(text, text, text, jsonb, jsonb, uuid, uuid) to anon, authenticated;
