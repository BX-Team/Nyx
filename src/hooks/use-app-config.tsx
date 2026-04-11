import { listen } from '@tauri-apps/api/event';
import React, { createContext, type ReactNode, useContext } from 'react';
import { toast } from 'sonner';
import useSWR from 'swr';
import { getAppConfig, patchAppConfig as patch } from '@/utils/ipc';

interface AppConfigContextType {
  appConfig: AppConfig | undefined;
  mutateAppConfig: () => void;
  patchAppConfig: (value: Partial<AppConfig>) => Promise<void>;
}

const AppConfigContext = createContext<AppConfigContextType | undefined>(undefined);

export const AppConfigProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const { data: appConfig, mutate: mutateAppConfig } = useSWR('getConfig', () => getAppConfig());

  const patchAppConfig = async (value: Partial<AppConfig>): Promise<void> => {
    try {
      await patch(value);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      mutateAppConfig();
    }
  };

  React.useEffect(() => {
    const unlisten = listen('app-config-updated', () => mutateAppConfig());
    return (): void => {
      unlisten.then(fn => fn());
    };
  }, [mutateAppConfig]);

  return (
    <AppConfigContext.Provider value={{ appConfig, mutateAppConfig, patchAppConfig }}>
      {children}
    </AppConfigContext.Provider>
  );
};

export const useAppConfig = (): AppConfigContextType => {
  const context = useContext(AppConfigContext);
  if (context === undefined) {
    throw new Error('useAppConfig must be used within an AppConfigProvider');
  }
  return context;
};
