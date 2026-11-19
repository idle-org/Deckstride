// Append to a temporary copy of the concept. Wait for the asynchronous PASS/FAIL report.
window.addEventListener('load', async () => {
 const results=[];const check=(value,label)=>{if(!value)throw Error(label);results.push(label);};
 const wait=()=>new Promise(resolve=>setTimeout(resolve,750));
 const pointer=(target,y)=>target.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,clientX:300,clientY:y}));
 const original=localStorage.getItem(layoutStorageKey);
 try {
  layout=structuredClone(layoutDefaults);applyLayout();
  const main=document.querySelector('main');const initial=main.clientHeight;
  $('hideHeader').click();$('hideFooter').click();
  check($('presenterHeader').hidden&&$('presenterFooter').hidden,'Both bars can be hidden');
  check(main.clientHeight>initial,'Hidden header releases height');
  check(layout.elapsed.show&&layout.clock.show,'Clock preferences preserved while header hidden');
  pointer(document.body,2);check(!$('presenterHeader').hidden&&$('presenterHeader').classList.contains('peek'),'Top edge reveals header');
  check(main.clientHeight>initial,'Reveal overlays without taking layout space');
  check(!layout.chrome.header,'Transient reveal does not pin header');
  pointer($('presenterHeader'),80);await wait();check(!$('presenterHeader').hidden,'Header remains while pointer is on it');
  pointer(document.body,innerHeight/2);await wait();check($('presenterHeader').hidden,'Header hides after leaving');
  pointer(document.body,innerHeight-2);check(!$('presenterFooter').hidden,'Bottom edge reveals footer');
  $('next').focus();pointer(document.body,innerHeight/2);await wait();check(!$('presenterFooter').hidden,'Keyboard focus keeps revealed bar usable');
  $('next').blur();await wait();check($('presenterFooter').hidden,'Footer hides after focus leaves');
  document.body.dispatchEvent(new MouseEvent('contextmenu',{bubbles:true,cancelable:true,clientX:innerWidth-2,clientY:innerHeight-2}));
  check(!$('chromeMenu').hidden&&$('contextHeader').textContent==='Show header','Right-click offers recovery');
  const r=$('chromeMenu').getBoundingClientRect();check(r.right<=innerWidth&&r.bottom<=innerHeight,'Context menu stays inside viewport');
  $('contextBoth').click();check(!$('presenterHeader').hidden&&!$('presenterFooter').hidden&&layout.chrome.header&&layout.chrome.footer,'Context menu restores both bars');
  setChromePinned('header',false);setChromePinned('footer',false);saveLayout('project');layout=structuredClone(layoutDefaults);restoreLayout();
  check(!layout.chrome.header&&!layout.chrome.footer,'Hidden bars round-trip per project');
  saveLayout('global');check(!readLayouts().global.chrome.header,'Global save includes visibility');
  const old=structuredClone(layoutDefaults);delete old.chrome;check(validateLayout(old).chrome.header&&validateLayout(old).chrome.footer,'Older records migrate to visible bars');
  let rejected=false;try{validateLayout({...old,chrome:{header:'false',footer:true}});}catch{rejected=true;}check(rejected,'Malformed chrome rejected');
  pointer(document.body,2);$('hideHeader').click();check(layout.chrome.header&&!$('presenterHeader').classList.contains('peek'),'Revealed bar can be pinned');
  const report=document.createElement('pre');report.style='position:fixed;inset:0;background:#102820;color:white;padding:24px;z-index:100;font:16px monospace;overflow:auto';report.textContent=`PASS · ${results.length} browser checks\n\n${results.join('\n')}`;document.body.append(report);
 }catch(error){document.body.textContent='FAIL: '+error.message;}
 finally{if(original===null)localStorage.removeItem(layoutStorageKey);else localStorage.setItem(layoutStorageKey,original);}
});
