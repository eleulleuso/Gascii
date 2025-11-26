# Game Performance Optimization Instructions

## Electron API Usage
- **BrowserWindow**: Optimize `BrowserWindow` settings for performance. Use `webPreferences.contextIsolation: true` for security and `nodeIntegration: false` in the renderer process.
- **Hardware Acceleration**: Use `app.disableHardwareAcceleration()` only if necessary for stability on specific systems, but prioritize a performant `BrowserWindow` for a smooth experience.
- **Main vs. Renderer Process**: Offload heavy computations (e.g., audio processing, data parsing) to the **main process** or a dedicated worker thread to prevent the renderer process from freezing. The renderer process should primarily handle UI updates and user input.

## Node.js Performance Best Practices
- **Asynchronous Operations**: All file I/O operations (like parsing beatmap files) must be handled asynchronously using `fs.promises` to avoid blocking the event loop.
- **Stream Processing**: For large files, use Node.js streams to process data chunk-by-chunk rather than loading the entire file into memory at once.

## React Optimization
- **Memoization**: Use `React.memo`, `useMemo`, and `useCallback` to prevent unnecessary re-renders of components, especially for the game canvas and UI elements.
- **Virtualization**: If displaying long lists (e.g., song selection or high scores), use a virtualization library to render only the visible items.