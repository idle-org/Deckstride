// Manual browser regression harness: append to a temporary copy of the HTML concept.
// Run only in an isolated browser profile. It restores the original storage record.
window.addEventListener('load', () => {
    const results = [];
    const originalGet = Storage.prototype.getItem;
    const originalSet = Storage.prototype.setItem;
    let original;
    const check = (condition, label) => {
        if (!condition) throw Error(label);
        results.push(label);
    };
    try {
        original = localStorage.getItem(layoutStorageKey);
        localStorage.removeItem(layoutStorageKey);
        restoreLayout();
        check(layout.notes.size === 28, 'Built-in default without saved settings');
        layout.notes.size = 42;
        saveLayout('global');
        check(readLayouts().global.notes.size === 42, 'All-project default round-trips');
        layout.notes.size = 60;
        layout.clock.show = false;
        saveLayout('project');
        layout = structuredClone(layoutDefaults);
        restoreLayout();
        check(layout.notes.size === 60 && !layout.clock.show, 'Project size and visibility restored');
        $('demoProject').value = 'workshop';
        $('demoProject').dispatchEvent(new Event('change'));
        check(layout.notes.size === 42 && layout.clock.show, 'Other project inherits global default');
        layout.notes.size = 35;
        saveLayout('global');
        $('demoProject').value = 'ideas';
        $('demoProject').dispatchEvent(new Event('change'));
        check(layout.notes.size === 60, 'Global save preserves project override');
        $('useGlobalLayout').click();
        check(layout.notes.size === 35 && !readLayouts().projects.ideas, 'Remove override restores global default');
        layout.notes.size = 55;
        restoreLayout();
        check(layout.notes.size === 35, 'Unsaved edits are not persisted');
        const saved = localStorage.getItem(layoutStorageKey);
        Storage.prototype.setItem = () => { throw Error('Quota exceeded'); };
        saveLayout('project');
        check($('layoutSaveStatus').textContent.startsWith('Could not save'), 'Write failure never reports success');
        Storage.prototype.setItem = originalSet;
        check(localStorage.getItem(layoutStorageKey) === saved, 'Failed save preserves stored data');
        for (const invalid of ['{', '{"version":2,"global":null,"projects":{}}', JSON.stringify({version:1,global:{},projects:{}})]) {
            localStorage.setItem(layoutStorageKey, invalid);
            restoreLayout();
            check(layout.notes.size === 28 && $('layoutSaveStatus').textContent.includes('could not be read'), 'Invalid/unsupported record falls back visibly');
            saveLayout('global');
            check(localStorage.getItem(layoutStorageKey) === invalid, 'Invalid saved data is not overwritten');
        }
        const invalidWidget = structuredClone(layoutDefaults);
        invalidWidget.clock.size = 999;
        let rejected = false;
        try { validateLayout(invalidWidget); } catch { rejected = true; }
        check(rejected, 'Out-of-range widget rejected');
        Storage.prototype.getItem = () => { throw Error('Storage denied'); };
        restoreLayout();
        check(layout.notes.size === 28, 'Unavailable storage keeps editor usable');
        Storage.prototype.getItem = originalGet;
        const report = document.createElement('pre');
        report.style = 'position:fixed;inset:0;background:#102820;color:white;padding:24px;z-index:100;font:16px monospace;overflow:auto';
        report.textContent = `PASS · ${results.length} browser checks\n\n${results.join('\n')}`;
        document.body.append(report);
    } catch (error) {
        document.body.textContent = 'FAIL: ' + error.message;
    } finally {
        Storage.prototype.getItem = originalGet;
        Storage.prototype.setItem = originalSet;
        if (original === null) localStorage.removeItem(layoutStorageKey);
        else if (original !== undefined) localStorage.setItem(layoutStorageKey, original);
    }
});
