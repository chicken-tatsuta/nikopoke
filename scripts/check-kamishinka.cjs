#!/usr/bin/env node
// Check the actual browser WASM wire format, including JSON transport used by PeerJS.
const fs = require('node:fs');
const path = require('node:path');
const assert = require('node:assert/strict');
const root = path.resolve(__dirname, '..');
const plain = (value) => value instanceof Map ? Object.fromEntries([...value].map(([k,v]) => [k, plain(v)]))
  : Array.isArray(value) ? value.map(plain) : value && typeof value === 'object'
    ? Object.fromEntries(Object.entries(value).map(([k,v]) => [k,plain(v)])) : value;
(async () => {
  const wasm = await import(path.join(root, 'engine-rust/pkg/engine_rust.js'));
  await wasm.default({ module_or_path: fs.readFileSync(path.join(root,'engine-rust/pkg/engine_rust_bg.wasm')) });
  const make = (species) => wasm.createCreature(species,{ item:'kamidama',moves:['tackle'],ability:'none' });
  let state = wasm.createBattleState(['host','guest'].map(id => ({id,name:id,activeSlot:0,team:[make('nisiki'),make('ayuma')]})));
  assert(state.players.every(p=>p.team.every(c=>c.item==='kamidama')));
  const hp = state.players.map(p=>p.team[0].maxHp);
  const actions = ['host','guest'].map(playerId=>({type:'move',playerId,moveId:'tackle',kamishinkaForm:'kami_nisiki'}));
  state = wasm.stepBattle(state,actions,{recordHistory:false});
  assert(state.players.every(p=>p.kamishinkaUsed && p.team[0].kamishinka.formId==='kami_nisiki'));
  assert.deepEqual(state.players.map(p=>p.team[0].maxHp),hp);
  state=JSON.parse(JSON.stringify(plain(state)));
  state=wasm.stepBattle(state,['host','guest'].map(playerId=>({type:'switch',playerId,slot:1})),{recordHistory:false});
  const illegal=['host','guest'].flatMap(playerId=>Array(2).fill({type:'move',playerId,moveId:'tackle',kamishinkaForm:'kami_ayuma'}));
  state=wasm.stepBattle(state,illegal,{recordHistory:false});
  assert(state.players.every(p=>p.kamishinkaUsed && !p.team[1].kamishinka));
  assert.equal(state.log.filter(l=>l.includes('カミシンカした')).length,2);
  state=wasm.createBattleState(state.players);
  assert(state.players.every(p=>!p.kamishinkaUsed && p.team.every(c=>!c.kamishinka && c.item==='kamidama')));
  const registry=JSON.parse(fs.readFileSync(path.join(root,'engine-rust/data/kamishinka.json')));
  assert.equal(registry.length,8);
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(root,'frontend/public/data/items.json'))).kamidama,
    JSON.parse(fs.readFileSync(path.join(root,'engine-rust/data/items.json'))).kamidama);
  console.log('Kamishinka WASM/JSON checks passed: duplicate items, both players, HP, transport, repeated requests, reset, data.');
})().catch(e=>{console.error(e);process.exitCode=1;});
