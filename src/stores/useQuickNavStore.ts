import { create } from "zustand";

interface QuickNavState {
  quickSwitcherOpen: boolean;
  commandPaletteOpen: boolean;
  openQuickSwitcher: () => void;
  closeQuickSwitcher: () => void;
  openCommandPalette: () => void;
  closeCommandPalette: () => void;
  toggleQuickSwitcher: () => void;
  toggleCommandPalette: () => void;
}

export const useQuickNavStore = create<QuickNavState>((set) => ({
  quickSwitcherOpen: false,
  commandPaletteOpen: false,
  openQuickSwitcher: () => set({ quickSwitcherOpen: true }),
  closeQuickSwitcher: () => set({ quickSwitcherOpen: false }),
  openCommandPalette: () => set({ commandPaletteOpen: true }),
  closeCommandPalette: () => set({ commandPaletteOpen: false }),
  toggleQuickSwitcher: () =>
    set((s) => ({ quickSwitcherOpen: !s.quickSwitcherOpen })),
  toggleCommandPalette: () =>
    set((s) => ({ commandPaletteOpen: !s.commandPaletteOpen })),
}));
