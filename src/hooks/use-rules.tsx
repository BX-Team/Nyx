import { listen } from '@tauri-apps/api/event';
import React, { createContext, type ReactNode, useContext } from 'react';
import useSWR from 'swr';
import { mihomoRules } from '@/utils/ipc';

interface RulesContextType {
  rules: ControllerRules | undefined;
  mutate: () => void;
}

const RulesContext = createContext<RulesContextType | undefined>(undefined);

export const RulesProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const { data: rules, mutate } = useSWR<ControllerRules>('mihomoRules', mihomoRules, {
    errorRetryInterval: 200,
    errorRetryCount: 10,
  });

  React.useEffect(() => {
    const u1 = listen('rules-updated', () => mutate());
    const u2 = listen('core-started', () => mutate());
    return (): void => {
      u1.then(f => f());
      u2.then(f => f());
    };
  }, [mutate]);

  return <RulesContext.Provider value={{ rules, mutate }}>{children}</RulesContext.Provider>;
};

export const useRules = (): RulesContextType => {
  const context = useContext(RulesContext);
  if (context === undefined) {
    throw new Error('useRules must be used within an RulesProvider');
  }
  return context;
};
