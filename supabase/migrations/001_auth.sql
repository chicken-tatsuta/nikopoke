create table if not exists public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  username text unique not null,
  win_count integer not null default 0,
  loss_count integer not null default 0,
  current_deck jsonb,
  saved_decks jsonb not null default '[]'::jsonb,
  created_at timestamptz not null default now()
);

alter table public.profiles enable row level security;

drop policy if exists "Anyone can read profiles" on public.profiles;
create policy "Anyone can read profiles"
  on public.profiles
  for select
  to anon, authenticated
  using (true);

drop policy if exists "Users can update own profile" on public.profiles;
create policy "Users can update own profile"
  on public.profiles
  for update
  to authenticated
  using (auth.uid() = id)
  with check (auth.uid() = id);

drop policy if exists "Users can insert own profile" on public.profiles;
create policy "Users can insert own profile"
  on public.profiles
  for insert
  to authenticated
  with check (auth.uid() = id);

alter table public.battle_records
  add column if not exists host_user_id uuid references auth.users(id) on delete set null,
  add column if not exists guest_user_id uuid references auth.users(id) on delete set null;

create or replace function public.handle_new_user_profile()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
declare
  base_username text;
  final_username text;
begin
  base_username := nullif(new.raw_user_meta_data->>'username', '');
  if base_username is null then
    base_username := nullif(split_part(new.email, '@', 1), '');
  end if;
  if base_username is null then
    base_username := 'trainer';
  end if;

  final_username := base_username;
  if exists (select 1 from public.profiles where username = final_username) then
    final_username := base_username || '_' || substr(new.id::text, 1, 8);
  end if;

  insert into public.profiles (id, username)
  values (new.id, final_username)
  on conflict (id) do nothing;

  return new;
end;
$$;

drop trigger if exists on_auth_user_created_profile on auth.users;
create trigger on_auth_user_created_profile
  after insert on auth.users
  for each row execute function public.handle_new_user_profile();

create or replace function public.apply_battle_record_profile_counts()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  if new.winner = 'host' then
    if new.host_user_id is not null then
      update public.profiles
      set win_count = win_count + 1
      where id = new.host_user_id;
    end if;

    if new.guest_user_id is not null then
      update public.profiles
      set loss_count = loss_count + 1
      where id = new.guest_user_id;
    end if;
  elsif new.winner = 'guest' then
    if new.guest_user_id is not null then
      update public.profiles
      set win_count = win_count + 1
      where id = new.guest_user_id;
    end if;

    if new.host_user_id is not null then
      update public.profiles
      set loss_count = loss_count + 1
      where id = new.host_user_id;
    end if;
  end if;

  return new;
end;
$$;

drop trigger if exists on_battle_record_insert_profile_counts on public.battle_records;
create trigger on_battle_record_insert_profile_counts
  after insert on public.battle_records
  for each row execute function public.apply_battle_record_profile_counts();
