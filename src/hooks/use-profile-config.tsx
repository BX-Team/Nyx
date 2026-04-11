import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type React from 'react';
import { createContext, type ReactNode, useCallback, useContext, useEffect, useState } from 'react';
import { toast } from 'sonner';
import useSWR from 'swr';
import {
  addProfileItem as add,
  changeCurrentProfile as change,
  getProfileConfig,
  removeProfileItem as remove,
  setProfileConfig as set,
  updateProfileItem as update,
} from '@/utils/ipc';

interface ProfileConfigContextType {
  profileConfig: ProfileConfig | undefined;
  setProfileConfig: (config: ProfileConfig) => Promise<void>;
  mutateProfileConfig: () => void;
  addProfileItem: (item: Partial<ProfileItem>) => Promise<void>;
  updateProfileItem: (item: ProfileItem) => Promise<void>;
  removeProfileItem: (id: string) => Promise<void>;
  changeCurrentProfile: (id: string) => Promise<void>;
  hwidLimitError: string | null;
  clearHwidLimitError: () => void;
}

const ProfileConfigContext = createContext<ProfileConfigContextType | undefined>(undefined);

export const ProfileConfigProvider: React.FC<{ children: ReactNode }> = ({ children }) => {
  const { data: profileConfig, mutate: mutateProfileConfig } = useSWR('getProfileConfig', () => getProfileConfig());
  const [hwidLimitError, setHwidLimitError] = useState<string | null>(null);

  const setHwidLimitErrorFromMessage = useCallback((message: string): void => {
    const match = message.match(/HWID_LIMIT:(.*)/);
    if (match) {
      setHwidLimitError(match[1].trim());
      return;
    }
    setHwidLimitError(message.trim());
  }, []);

  const clearHwidLimitError = useCallback(() => setHwidLimitError(null), []);

  const setProfileConfig = async (config: ProfileConfig): Promise<void> => {
    try {
      await set(config);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      mutateProfileConfig();
      invoke('update_tray_icon').catch(() => {});
    }
  };

  const addProfileItem = async (item: Partial<ProfileItem>): Promise<void> => {
    try {
      await add(item);
    } catch (e) {
      if (`${e}`.includes('HWID_LIMIT')) {
        setHwidLimitErrorFromMessage(`${e}`);
      } else {
        toast.error(`${e}`);
      }
    } finally {
      mutateProfileConfig();
      invoke('update_tray_icon').catch(() => {});
    }
  };

  const removeProfileItem = async (id: string): Promise<void> => {
    try {
      await remove(id);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      mutateProfileConfig();
      invoke('update_tray_icon').catch(() => {});
    }
  };

  const updateProfileItem = async (item: ProfileItem): Promise<void> => {
    try {
      await update(item);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      mutateProfileConfig();
      invoke('update_tray_icon').catch(() => {});
    }
  };

  const changeCurrentProfile = async (id: string): Promise<void> => {
    try {
      await change(id);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      mutateProfileConfig();
      invoke('update_tray_icon').catch(() => {});
    }
  };

  useEffect(() => {
    const handleProfileConfigUpdated = (): void => {
      mutateProfileConfig();
    };
    const handleShowHwidLimitError = (_event: unknown, supportUrl = ''): void => {
      setHwidLimitErrorFromMessage(supportUrl);
    };

    const u1 = listen('profile-config-updated', handleProfileConfigUpdated);
    const u2 = listen<string>('show-hwid-limit-error', e => handleShowHwidLimitError(null, e.payload));
    return (): void => {
      u1.then(f => f());
      u2.then(f => f());
    };
  }, [mutateProfileConfig, setHwidLimitErrorFromMessage]);

  return (
    <ProfileConfigContext.Provider
      value={{
        profileConfig,
        setProfileConfig,
        mutateProfileConfig,
        addProfileItem,
        removeProfileItem,
        updateProfileItem,
        changeCurrentProfile,
        hwidLimitError,
        clearHwidLimitError,
      }}
    >
      {children}
    </ProfileConfigContext.Provider>
  );
};

export const useProfileConfig = (): ProfileConfigContextType => {
  const context = useContext(ProfileConfigContext);
  if (context === undefined) {
    throw new Error('useProfileConfig must be used within a ProfileConfigProvider');
  }
  return context;
};
