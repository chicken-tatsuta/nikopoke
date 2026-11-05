#!/usr/bin/env node
// Requires a running Vite dev server. Auth is mocked in this isolated browser only.
// PLAYWRIGHT_MODULE may point to an existing Playwright installation.
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root=path.resolve(__dirname,'../..');
const species=JSON.parse(fs.readFileSync(path.join(root,'engine-rust/data/species.json')));
const learnsets=JSON.parse(fs.readFileSync(path.join(root,'engine-rust/data/learnsets.json')));
const deck=['nisiki','ikkun','ayuma','morimori','haruta','makocchan'].map(speciesId=>({speciesId,moves:learnsets[speciesId].slice(0,4),ability:species[speciesId].abilities[0],item:null}));
(async()=>{
  const browser=await chromium.launch({headless:true, executablePath:process.env.CHROME_PATH});
  try {
    const context=await browser.newContext({viewport:{width:1280,height:900}});
    await context.route('**/src/contexts/AuthContext.tsx',route=>route.fulfill({contentType:'text/javascript',body:`
      const noop = async () => {};
      export const USERNAME_MAX_LENGTH=16;
      export function AuthProvider({children}) {return children;}
      export function useAuth() {return {session:{},user:null,profile:null,loading:false,updateProfile:noop,signIn:noop,signOut:noop,signUp:noop,refreshProfile:noop};}
    `}));
    await context.addInitScript(deck=>{
      localStorage.setItem('savedDeck',JSON.stringify(deck));
      sessionStorage.setItem('battleMode','ai');
    },deck);
    const page=await context.newPage();
    const errors=[];page.on('pageerror',e=>errors.push(e.message));
    const origin=process.env.TEST_ORIGIN || 'http://127.0.0.1:5173';
    await page.goto(origin+'/deck-builder');
    for(let i=0;i<2;i++) {
      await page.getByRole('button',{name:'道具',exact:true}).nth(i).click();
      await page.getByRole('button',{name:'SUP',exact:true}).click();
      const orb=page.getByRole('button',{name:/カミダマ/});assert(await orb.isEnabled());await orb.click();
      await page.getByRole('button',{name:'保存',exact:true}).click();
    }
    const saved=await page.evaluate(()=>JSON.parse(localStorage.getItem('savedDeck')));
    assert.equal(saved.filter(c=>c.item==='kamidama').length,2);
    await page.evaluate(saved=>{
      const local=saved.slice(0,3).map(c=>({...c,moves:['tackle']}));
      const opponent=[{speciesId:'reosan',ability:'none',moves:['growl'],item:null}];
      sessionStorage.setItem('playerDeck',JSON.stringify(saved));
      sessionStorage.setItem('selectedPlayerDeck',JSON.stringify(local));
      sessionStorage.setItem('selectedOpponentDeck',JSON.stringify(opponent));
    },saved);
    await page.goto(origin+'/battle');
    const toggle=page.getByRole('button',{name:'カミシンカ',exact:true});
    await toggle.waitFor();await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent.trim()==='カミシンカ'&&!b.disabled));
    await toggle.click();assert.equal(await page.getByRole('button',{name:/予約中/}).getAttribute('aria-pressed'),'true');
    await page.getByRole('button',{name:/予約中/}).click();assert.equal(await toggle.getAttribute('aria-pressed'),'false');
    await toggle.click();
    await page.locator('.battle-move-grid button').first().click();
    await page.mouse.move(1200,20);
    await page.locator('.kami-overlay').waitFor({timeout:30000});
    const overlay=await page.locator('.kami-overlay').boundingBox();
    assert.equal(overlay.width,1280);assert.equal(overlay.height,900);
    assert.equal(await page.locator('.battle-move-grid button:enabled').count(),0);
    assert(await page.getByRole('button',{name:'ニキダン',exact:true}).isDisabled());
    await page.waitForTimeout(350);
    await page.screenshot({path:'/tmp/kamishinka-animation.png'});
    await page.locator('.kami-overlay').waitFor({state:'hidden',timeout:15000});
    await page.getByRole('button',{name:'カミシンカ 使用済み',exact:true}).waitFor();
    assert(await page.getByRole('button',{name:'カミシンカ 使用済み',exact:true}).isDisabled());
    await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent.trim()==='ニキダン'&&!b.disabled));
    assert(await page.getByRole('heading',{name:'メガニシキ',exact:true}).isVisible());
    await page.screenshot({path:'/tmp/kamishinka-battle.png'});
    for (const name of ['いっくん','メガニシキ']) {
      await page.getByRole('button',{name:'ニキダン',exact:true}).click();
      await page.getByRole('button',{name:new RegExp(name)}).first().click();
      await page.getByRole('button',{name:'交代する',exact:true}).click();
      await page.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent.trim()==='ニキダン'&&!b.disabled));
      assert(await page.getByRole('heading',{name,exact:true}).isVisible());
      assert(await page.getByRole('button',{name:'カミシンカ 使用済み',exact:true}).isDisabled());
    }
    await page.setViewportSize({width:390,height:844});await page.screenshot({path:'/tmp/kamishinka-mobile.png'});
    assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth>window.innerWidth),false);
    assert.deepEqual(errors,[]);
    console.log('UI checks passed: two held items, reservation/cancel, animation input lock, evolved name, used button, switch persistence, mobile layout.');
  } finally {await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
