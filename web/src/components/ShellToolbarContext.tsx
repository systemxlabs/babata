import { createContext, useContext } from 'react';

interface ShellToolbarContextValue {
  autoRefresh: boolean;
  registerRefreshHandler: (handler: (() => Promise<void> | void) | null) => void;
}

const ShellToolbarContext = createContext<ShellToolbarContextValue | null>(null);

export const ShellToolbarProvider = ShellToolbarContext.Provider;

export function useShellToolbar() {
  return useContext(ShellToolbarContext);
}
