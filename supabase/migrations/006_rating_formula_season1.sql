drop function if exists public.record_battle_result(text, text, text, jsonb, jsonb, uuid, uuid);
drop function if exists public.apply_profile_rating_result(uuid, uuid);
drop function if exists public.rating_delta(integer, integer, numeric, integer);

create or replace function public.rating_delta(
  p_rating integer,
  p_opponent_rating integer,
  p_score numeric,
  p_total_match_count integer default 0
)
returns integer
language sql
immutable
as $$
  select round(40 * (p_score - (1 / (1 + power(10, ((p_opponent_rating - p_rating)::numeric / 400))))))::integer
$$;

create or replace function public.rating_upset_bonus(
  p_rating integer,
  p_opponent_rating integer
)
returns integer
language sql
immutable
as $$
  select case
    when p_opponent_rating > p_rating then least(10, greatest(2, ceiling((p_opponent_rating - p_rating)::numeric / 50)::integer))
    else 0
  end
$$;

create or replace function public.apply_profile_rating_result(
  p_winner_user_id uuid,
  p_loser_user_id uuid,
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
  winner_rating integer;
  loser_rating integer;
  winner_elo_delta integer;
  loser_elo_delta integer;
  winner_bonus integer;
  loser_bonus integer;
  winner_delta integer;
  loser_delta integer;
begin
  out_winner_delta := 0;
  out_loser_delta := 0;
  out_winner_elo_delta := 0;
  out_loser_elo_delta := 0;
  out_winner_bonus := 0;
  out_loser_bonus := 0;

  if p_winner_user_id is null and p_loser_user_id is null then
    return;
  end if;

  if p_winner_user_id is not null then
    select rating into winner_rating from public.profiles where id = p_winner_user_id;
  end if;
  if p_loser_user_id is not null then
    select rating into loser_rating from public.profiles where id = p_loser_user_id;
  end if;

  winner_rating := coalesce(winner_rating, 1500);
  loser_rating := coalesce(loser_rating, 1500);

  winner_elo_delta := public.rating_delta(winner_rating, loser_rating, 1, 0);
  loser_elo_delta := public.rating_delta(loser_rating, winner_rating, 0, 0);

  winner_bonus := 0;
  if winner_rating < 1700 then
    winner_bonus := winner_bonus + 2 + 3;
  end if;
  winner_bonus := winner_bonus + public.rating_upset_bonus(winner_rating, loser_rating);

  loser_bonus := 0;
  if loser_rating < 1700 then
    loser_bonus := loser_bonus + 2;
  end if;
  if loser_rating < 1600 then
    loser_bonus := loser_bonus + 5;
  end if;

  winner_delta := winner_elo_delta + winner_bonus;
  loser_delta := least(0, loser_elo_delta + loser_bonus);

  if p_winner_user_id is not null then
    update public.profiles
    set
      win_count = win_count + 1,
      total_match_count = total_match_count + 1,
      rating = greatest(0, rating + winner_delta)
    where id = p_winner_user_id;
  end if;

  if p_loser_user_id is not null then
    update public.profiles
    set
      loss_count = loss_count + 1,
      total_match_count = total_match_count + 1,
      rating = greatest(0, rating + loser_delta)
    where id = p_loser_user_id;
  end if;

  out_winner_delta := winner_delta;
  out_loser_delta := loser_delta;
  out_winner_elo_delta := winner_elo_delta;
  out_loser_elo_delta := loser_elo_delta;
  out_winner_bonus := winner_delta - winner_elo_delta;
  out_loser_bonus := loser_delta - loser_elo_delta;
end;
$$;

create or replace function public.apply_battle_record_profile_counts()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if new.winner = 'host' then
    perform public.apply_profile_rating_result(new.host_user_id, new.guest_user_id);
  elsif new.winner = 'guest' then
    perform public.apply_profile_rating_result(new.guest_user_id, new.host_user_id);
  end if;

  return new;
end;
$$;

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
      out_winner_delta,
      out_loser_delta,
      out_winner_elo_delta,
      out_loser_elo_delta,
      out_winner_bonus,
      out_loser_bonus
    into wd, ld, we, le, wb, lb
    from public.apply_profile_rating_result(p_player_user_id, p_opponent_user_id);
  elsif p_winner_side = 'opponent' then
    select
      out_winner_delta,
      out_loser_delta,
      out_winner_elo_delta,
      out_loser_elo_delta,
      out_winner_bonus,
      out_loser_bonus
    into wd, ld, we, le, wb, lb
    from public.apply_profile_rating_result(p_opponent_user_id, p_player_user_id);
  end if;

  out_winner_delta := coalesce(wd, 0);
  out_loser_delta := coalesce(ld, 0);
  out_winner_elo_delta := coalesce(we, 0);
  out_loser_elo_delta := coalesce(le, 0);
  out_winner_bonus := coalesce(wb, 0);
  out_loser_bonus := coalesce(lb, 0);
end;
$$;

grant execute on function public.rating_delta(integer, integer, numeric, integer) to anon, authenticated;
grant execute on function public.rating_upset_bonus(integer, integer) to anon, authenticated;
grant execute on function public.apply_profile_rating_result(uuid, uuid) to anon, authenticated;
grant execute on function public.record_battle_result(text, text, text, jsonb, jsonb, uuid, uuid) to anon, authenticated;
