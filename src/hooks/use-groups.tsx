import { listen } from '@tauri-apps/api/event';
import React, { createContext, type ReactNode, useContext } from 'react';
import useSWR from 'swr';
import { mihomoGroups } from '@/utils/ipc';

interface GroupsContextType {
  groups: ControllerMixedGroup[] | undefined;
  mutate: () => void;
}

const GroupsContext = createContext<GroupsContextType | undefined>(undefined);

export const GroupsProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const { data: groups, mutate } = useSWR<ControllerMixedGroup[]>('mihomoGroups', mihomoGroups, {
    errorRetryInterval: 200,
    errorRetryCount: 10,
  });

  React.useEffect(() => {
    const u1 = listen('groups-updated', () => mutate());
    const u2 = listen('core-started', () => mutate());
    return (): void => {
      u1.then(f => f());
      u2.then(f => f());
    };
  }, [mutate]);

  return <GroupsContext.Provider value={{ groups, mutate }}>{children}</GroupsContext.Provider>;
};

export const useGroups = (): GroupsContextType => {
  const context = useContext(GroupsContext);
  if (context === undefined) {
    throw new Error('useGroups must be used within an GroupsProvider');
  }
  return context;
};
