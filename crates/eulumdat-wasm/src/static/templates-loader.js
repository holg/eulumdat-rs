// Templates WASM loader for lazy-loaded template content
// Lazy-loads the eulumdat-wasm-templates WASM module on demand

let templatesModule = null;
let templatesInitPromise = null;

async function initTemplates() {
    if (templatesModule) return templatesModule;
    if (templatesInitPromise) return templatesInitPromise;

    templatesInitPromise = (async () => {
        try {
            console.log('[Templates] Loading templates module...');
            const module = await import('./templates/eulumdat_wasm_templates.js');
            await module.default();
            templatesModule = module;
            console.log('[Templates] Module loaded successfully');
            return module;
        } catch (e) {
            console.error('[Templates] Failed to load:', e);
            templatesInitPromise = null;
            throw e;
        }
    })();

    return templatesInitPromise;
}

window.getTemplateContent = async function(id) {
    const module = await initTemplates();
    return module.get_template_content(id);
};

window.isTemplatesLoaded = function() {
    return templatesModule !== null;
};

window.preloadTemplates = async function() {
    await initTemplates();
};
