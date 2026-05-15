alter table public.profiles
  add column if not exists is_admin boolean not null default false;

revoke update on table public.profiles from authenticated;
grant update (id, username, current_deck, saved_decks) on table public.profiles to authenticated;

revoke insert on table public.profiles from authenticated;
grant insert (id, username, current_deck, saved_decks) on table public.profiles to authenticated;

create or replace function public.reset_season_ratings(p_rating integer default 1500)
returns integer
language plpgsql
security definer
set search_path = public
as $$
declare
  reset_count integer;
begin
  if auth.uid() is null then
    raise exception 'ログインが必要です。' using errcode = '42501';
  end if;

  if not exists (
    select 1
    from public.profiles
    where id = auth.uid()
      and is_admin = true
  ) then
    raise exception '管理者権限が必要です。' using errcode = '42501';
  end if;

  update public.profiles
  set rating = greatest(0, p_rating);

  get diagnostics reset_count = row_count;
  return reset_count;
end;
$$;

revoke all on function public.reset_season_ratings(integer) from public;
grant execute on function public.reset_season_ratings(integer) to authenticated;
