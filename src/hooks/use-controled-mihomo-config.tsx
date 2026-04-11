import { listen } from '@tauri-apps/api/event';
import React, { createContext, type ReactNode, useContext } from 'react';
import { toast } from 'sonner';
import useSWR from 'swr';
import { getControledMihomoConfig, patchControledMihomoConfig as patch } from '@/utils/ipc';

interface ControledMihomoConfigContextType {
  controledMihomoConfig: Partial<MihomoConfig> | undefined;
  mutateControledMihomoConfig: () => void;
  patchControledMihomoConfig: (value: Partial<MihomoConfig>) => Promise<void>;
}

const ControledMihomoConfigContext = createContext<ControledMihomoConfigContextType | undefined>(undefined);

export const ControledMihomoConfigProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const { data: controledMihomoConfig, mutate: mutateControledMihomoConfig } = useSWR('getControledMihomoConfig', () =>
    getControledMihomoConfig(),
  );

  const patchControledMihomoConfig = async (value: Partial<MihomoConfig>): Promise<void> => {
    try {
      await patch(value);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      mutateControledMihomoConfig();
    }
  };

  React.useEffect(() => {
    const u = listen('controled-mihomo-config-updated', () => mutateControledMihomoConfig());
    return (): void => {
      u.then(f => f());
    };
  }, [mutateControledMihomoConfig]);

  return (
    <ControledMihomoConfigContext.Provider
      value={{ controledMihomoConfig, mutateControledMihomoConfig, patchControledMihomoConfig }}
    >
      {children}
    </ControledMihomoConfigContext.Provider>
  );
};

export const useControledMihomoConfig = (): ControledMihomoConfigContextType => {
  const context = useContext(ControledMihomoConfigContext);
  if (context === undefined) {
    throw new Error('useControledMihomoConfig must be used within a ControledMihomoConfigProvider');
  }
  return context;
};
