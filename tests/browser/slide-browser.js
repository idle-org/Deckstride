// Append to a temporary HTML concept copy. Use an isolated Firefox profile allowing popups.
window.addEventListener('load', () => {
    const results = [];
    const check = (value, label) => { if (!value) throw Error(label); results.push(label); };
    const key = (value, doc = document, target = doc.body, options = {}) => target.dispatchEvent(new KeyboardEvent('keydown', {key:value,bubbles:true,cancelable:true,...options}));
    const role = (id, value) => { $('screenRole'+id).value=value; $('screenRole'+id).dispatchEvent(new Event('change')); };
    try {
        key('1'); key('2'); check(page === 0, 'Typing does not navigate'); key('Enter'); check(page === 11, 'Number + Enter jumps to slide 12');
        key('9'); key('9'); key('Enter'); check(page === 11 && !$('jumpIndicator').hidden, 'Invalid number keeps current slide');
        key('Escape'); check($('jumpIndicator').hidden, 'Escape cancels pending jump');
        key('1'); key('2'); key('Backspace'); key('Enter'); check(page === 0, 'Backspace edits pending number');
        key('2',document,$('topology')); key('Enter'); check(page === 0, 'Input controls do not trigger jumps');
        key('2',document,document.body,{ctrlKey:true}); key('Enter'); check(page === 0, 'Modified keys do not trigger jumps');
        blank=true;key('3');key('Enter');check(page===2 && blank, 'Direct jump preserves blanking');
        openGallery(); check(galleryWindow && !galleryWindow.closed, 'Separate popup opens');
        const doc=galleryWindow.document;
        check(doc.querySelectorAll('.thumbnail').length===12 && !doc.fullscreenElement, 'All thumbnails in a non-fullscreen popup');
        doc.querySelectorAll('.thumbnail')[7].click();check(gallery.selected===7 && page===2, 'Default click browses without navigation');
        doc.getElementById('changeOnClick').checked=true;doc.getElementById('changeOnClick').dispatchEvent(new Event('change'));check(page===2, 'Enabling click change does not activate selection');
        doc.querySelectorAll('.thumbnail')[8].click();check(page===8 && blank, 'Opt-in click changes slide and preserves blanking');
        doc.getElementById('changeOnClick').checked=false;doc.getElementById('changeOnClick').dispatchEvent(new Event('change'));doc.querySelectorAll('.thumbnail')[1].click();doc.getElementById('showSelected').click();check(page===1, 'Show selected explicitly navigates');
        key('1',doc);key('0',doc);key('Enter',doc);check(page===9, 'Number shortcut works in popup');
        doc.getElementById('thumbnailSize').value='350';doc.getElementById('thumbnailSize').dispatchEvent(new Event('input'));check(gallery.size===350 && doc.getElementById('galleryGrid').style.getPropertyValue('--thumb')==='350px', 'Thumbnail size adjustable');
        openDisplays();role(2,'presenter');$('apply').click();check(active.presenter.join(',')==='1,2' && active.presentation.join(',')==='3', 'Two presenter screens and one presentation');
        openDisplays();$('swap').click();$('apply').click();check(active.presenter.join(',')==='3' && active.presentation.join(',')==='1,2', 'Group swap mirrors presentation instead');
        openDisplays();role(3,'presentation');check($('apply').disabled, 'Cannot apply without a presenter');$('displayDialog').close();check(active.presenter.join(',')==='3', 'Cancel retains applied groups');
        gallery.monitor='3';$('topology').value='2';$('topology').dispatchEvent(new Event('change'));check(active.presenter.join(',')==='1' && active.presentation.join(',')==='2' && blank, 'Missing presenter falls back without role overlap');check(galleryTarget()===1 && gallery.monitor==='3', 'Missing browser monitor falls back without erasing preference');
        $('topology').value='3';$('topology').dispatchEvent(new Event('change'));check(active.presenter.join(',')==='3' && active.presentation.join(',')==='1,2', 'Preferred groups survive reconnect');
        galleryWindow.close();openGallery();check(!gallery.changeOnClick, 'Reopened browser defaults to safe browsing');galleryWindow.close();
        const report=document.createElement('pre');report.style='position:fixed;inset:0;background:#102820;color:white;padding:24px;z-index:100;font:16px monospace;overflow:auto';report.textContent=`PASS · ${results.length} browser checks\n\n${results.join('\n')}`;document.body.append(report);
    } catch(error) { if(galleryWindow&&!galleryWindow.closed)galleryWindow.close();document.body.textContent='FAIL: '+error.message; }
});
