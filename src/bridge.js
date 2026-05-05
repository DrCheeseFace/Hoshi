const invoke = window.__TAURI__.core.invoke;
const listen = window.__TAURI__.event.listen;

let currentSgfFilePath = null;

// --- 1. WINDOW CONTROLS & MANUAL DRAGGING ---
const header = document.getElementById('main-header');
if (header) {
    header.addEventListener('mousedown', (e) => {
        // Drag only if we didn't click a button, the info icon, OR the About Logo
        if (!e.target.closest('button') && !e.target.closest('.info-icon') && !e.target.closest('#btn-about')) {
            invoke('plugin:window|start_dragging');
        }
    });
}

// --- 2. ELECTRON API MOCK ---
window.electronAPI = {
    getSystemProfile: async () => await invoke('get_system_profile'),
    checkCudaInstalled: async () => await invoke('check_cuda_installed'),
    checkForUpdates: async () => await invoke('check_for_updates'),
    openExternal: (url) => invoke('open_external', { url: url }),
    chooseDownloadFolder: async () => await invoke('native_folder_dialog', { title: "Select Installation Folder" }),
    getAppBaseDir: async () => await invoke('get_app_base_dir'),
    resolveDestination: async (path) => await invoke('resolve_destination', { basePath: path }),
    onDownloadProgress: (callback) => listen('download-progress', (e) => callback(e.payload)),
    cancelDownload: async () => await invoke('cancel_download'),

    downloadKataGo: async (folder, engineUrl, networkUrl) => {
        return await invoke('download_katago', {
            targetFolder: folder,
            engineUrl: engineUrl,
            networkUrl: networkUrl
        });
    },

    minimizeWindow: () => invoke('plugin:window|minimize'),
    maximizeWindow: () => invoke('plugin:window|toggle_maximize'),
    closeWindow: () => invoke('plugin:window|close'),

    getDefaultEnginePaths: async () => {
        const paths = await invoke('get_default_engine_paths');
        return { exe: paths.exePath, net: paths.modelPath, cfg: paths.cfgPath || '' };
    },

    checkFileExists: async (path) => {
        if (!path) return false;
        return await invoke('file_exists', { path: path });
    },

    startEngine: async (paths) => {
        await invoke('stop_katago');

        // Strip any accidental quotes from user copy-pasting
        let cleanNet = paths.net ? paths.net.replace(/['"]+/g, '') : '';
        let args = ["analysis", "-model", cleanNet];

        if (paths.cfg && paths.cfg.trim() !== '') {
            args.push("-config", paths.cfg.replace(/['"]+/g, ''));
        }

        try {
            await invoke('start_katago', { exePath: paths.exe.replace(/['"]+/g, ''), args: args });
        } catch (e) {
            console.error("KataGo boot failed:", e);
            document.dispatchEvent(new CustomEvent('internal-katago-error', { detail: "engine_missing" }));
        }
    },

    stopEngine: async () => await invoke('stop_katago'),

    chooseEngineFile: async (curr) => (await invoke('native_open_dialog', { title: "Select Engine", fName: "Executable", fExt: "exe" })) || curr,
    chooseNetworkFile: async (curr) => (await invoke('native_open_dialog', { title: "Select Model", fName: "Model", fExt: "bin.gz" })) || curr,
    chooseConfigFile: async (curr) => (await invoke('native_open_dialog', { title: "Select Config", fName: "Config", fExt: "cfg" })) || curr,

    sendAnalysisQuery: (query) => invoke('send_katago_command', { command: JSON.stringify(query) }),

    onAnalysisDataBatch: (callback) => {
        listen('katago-stdout', (e) => {
            try { callback([JSON.parse(e.payload)]); } catch (err) {}
        });

        // Pipe KataGo's internal logs to the console AND to the UI for tuning
        listen('katago-stderr', (e) => {
            console.log("[KataGo Engine]:", e.payload);
            const msg = e.payload.toLowerCase();

            if (msg.includes('error') || msg.includes('fatal')) {
                console.error("KataGo Fatal:", e.payload);
            } else if (msg.includes('tuning') || msg.includes('testing') || msg.includes('initialized')) {
                // Send background engine status updates to the UI
                document.dispatchEvent(new CustomEvent('internal-katago-status', { detail: e.payload }));
            }
        });

        document.addEventListener('internal-katago-error', (e) => callback([{ error: e.detail }]));
    },

    openSgf: async () => {
        const file = await invoke('native_open_dialog', { title: "Open SGF", fName: "SGF", fExt: "sgf" });
        if (file) {
            currentSgfFilePath = file;
            const content = await invoke('read_file', { path: file });
            document.dispatchEvent(new CustomEvent('internal-sgf-data', { detail: content }));
            document.dispatchEvent(new Event('internal-file-linked'));
        }
    },

    onSgfData: (callback) => {
        document.addEventListener('internal-sgf-data', (e) => callback(e.detail));
        listen('sgf-data', (e) => {
            callback(e.payload);
            document.dispatchEvent(new Event('internal-file-linked'));
        });
        invoke('get_cold_boot_sgf').then(data => { if (data) { callback(data); document.dispatchEvent(new Event('internal-file-linked')); } });
    },

    getFilePath: (file) => file.name,
    setFilePath: (path) => { currentSgfFilePath = path; document.dispatchEvent(new Event('internal-file-linked')); },
    resetFilePath: () => { currentSgfFilePath = null; },
    onFileLinked: (callback) => document.addEventListener('internal-file-linked', callback),

    saveSgf: async (data, defaultName) => {
        if (!currentSgfFilePath) { window.electronAPI.saveAsSgf(data, defaultName); }
        else { await invoke('write_file', { path: currentSgfFilePath, content: data }); }
    },

    saveAsSgf: async (data, defaultName) => {
        const file = await invoke('native_save_dialog', { title: "Save SGF", defPath: defaultName || "", fName: "SGF", fExt: "sgf" });
        if (file) {
            currentSgfFilePath = file;
            await invoke('write_file', { path: file, content: data });
            document.dispatchEvent(new Event('internal-file-linked'));
        }
    }
};

// --- 3. NATIVE DRAG AND DROP ---
listen('tauri://drag-enter', () => {
    document.getElementById('drag-overlay')?.classList.add('active');
});

listen('tauri://drag-leave', () => {
    document.getElementById('drag-overlay')?.classList.remove('active');
});

listen('tauri://drag-drop', async (e) => {
    document.getElementById('drag-overlay')?.classList.remove('active');

    const payload = e.payload;
    const paths = payload?.paths ? payload.paths : (Array.isArray(payload) ? payload : []);

    if (paths.length > 0 && paths[0].toLowerCase().endsWith('.sgf')) {
        const filePath = paths[0];

        try {
            const content = await invoke('read_file', { path: filePath });

            currentSgfFilePath = filePath;

            document.dispatchEvent(new CustomEvent('internal-sgf-data', { detail: content }));
            document.dispatchEvent(new Event('internal-file-linked'));
        } catch (err) {
            console.error("Failed to read dropped SGF:", err);
        }
    }
});
