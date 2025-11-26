---
description: GitHub Copilot instructions for the Prg (pin rhythm game) project.
---

# Prg (Pin Rhythm Game) - Copilot Instructions

## 📚 Project Overview
Pin rhythm game inspired by OSU! Built with **Electron + Next.js hybrid architecture**. Players hit pins to rhythm with bonus scoring for hitting existing pins. Uses OSU! `.osz` beatmap files for game content.

## 🏗️ Architecture Fundamentals

### Electron Process Structure
- **Main Process** (`src/main/`): Node.js backend, beatmap parsing, game state management
- **Renderer Process** (`src/renderer/`): Next.js React frontend with game UI
- **Preload** (`src/preload/`): Secure bridge using `contextBridge` API
- **Shared** (`src/shared/`): Type definitions and utilities for both processes

### Critical Patterns
- **Singleton Engines**: All game engines (`GameEngine`, `PhysicsEngine`, `AudioManager`) use singleton pattern
- **IPC Communication**: Type-safe main ↔ renderer communication via `src/shared/ipc/index.ts`
- **Scene Management**: Centralized scene routing through `SceneManager.tsx` context
- **"GIGA-CHAD" Comments**: Project convention for marking critical/optimized code sections

## 🔧 Development Workflows

### Essential Commands
```bash
pnpm run dev          # Concurrent Next.js + Electron development
pnpm run dev:next     # Next.js only (port 5173)
pnpm run dev:electron # Electron main process only
pnpm run build        # Full production build
```

### IPC Communication Pattern
Always use the typed IPC client from `src/renderer/utils/ipc-client.ts`:
```typescript
// ✅ Correct - Type-safe IPC
const beatmaps = await ipcClient.invoke('beatmap:list')

// ❌ Wrong - Direct electron API calls
window.electronAPI.invoke('beatmap:list')
```

## 🎮 Game Engine Architecture

### Engine Hierarchy
1. **GameEngine** (`src/renderer/game/GameEngine.ts`) - Master coordinator
2. **PhysicsEngine** - Pin physics and collision detection
3. **RenderEngine** - PIXI.js rendering pipeline
4. **AudioManager** - Audio playback and timing
5. **JudgmentManager** - Hit detection and scoring

### Beatmap Processing
- **Parsing**: Use `BeatmapParserService.ts` with `osu-parsers` library
- **Storage**: Beatmaps stored in `~/.prg/charts/` directory
- **Loading**: Via `BeatmapService.ts` with automatic `.osz` extraction

## 📁 File Organization Rules
- **Game Logic**: Core engines in `src/renderer/game/engines/`
- **IPC Handlers**: Main process handlers in `src/main/handlers/`
- **Scene Components**: React scenes in `src/renderer/scenes/`
- **Type Definitions**: Shared types in `src/shared/types/`

## 🚨 Critical Implementation Notes
- **userData Path**: Set to `~/.prg` before `app.ready` (see `src/main/index.ts`)
- **Security**: All IPC channels must be registered in preload script
- **Scene Transitions**: Use `SceneManager` context, never direct navigation
- **Logging**: Use structured logger from `src/shared/logger.ts`
- **Module Aliases**: TypeScript paths configured for `@main/*` and `@renderer/*`

---

> ⚠️ This hybrid Electron + Next.js architecture requires careful process separation. Never mix main/renderer APIs directly.