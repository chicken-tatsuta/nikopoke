# Battle Tools

The frontend exposes four tools at `/tools/damage`, `/tools/party`,
`/tools/speed`, and `/tools/matchup`. Access follows the existing login policy.
The home navigation links to `/tools`.

The imported tools live in `frontend/src/tools`. MUI is scoped with a local
ThemeProvider. The application retains its existing Tailwind styling.

`data/catalog.ts` adapts `frontend/public/data/species.json`, `moves.json`, and
`learnsets.json` at build time. Run the existing data sync and rebuild after
changing the battle catalog. Species IDs come from Nikipoke. The imported
calculator internally identifies moves by Japanese name; aliases with the same
name share their learnset targets. Presets come from the main app and are
filtered against the current learnsets.

EV controls use 0..32 per stat and the matchup editor enforces a total of 66.
Stat calculation adds EVs directly, matching `engine-rust/src/core/factory.rs`.

The damage calculator and matchup simulator retain the imported TypeScript
calculation engine. Move effect annotations come from the imported move data;
not all Rust battle mechanics, abilities, or newer move effects are modeled.
The matchup view labels results as estimates. A future Rust engine adapter is
needed for exact battle parity. Simulations do not record wins or usage in
Supabase.

Validation:

```sh
cd frontend
node scripts/check-tools.mjs
npm run build
```
