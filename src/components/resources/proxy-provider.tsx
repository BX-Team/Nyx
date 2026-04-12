import { listen } from '@tauri-apps/api/event';
import dayjs from 'dayjs';
import { FilePenLine, FileText, RefreshCcw } from 'lucide-react';
import { Fragment, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import useSWR from 'swr';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { calcTraffic } from '@/utils/calc';
import { getHash } from '@/utils/hash';
import { getRuntimeConfig, mihomoProxyProviders, mihomoUpdateProxyProviders } from '@/utils/ipc';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';
import Viewer from './viewer';

const ProxyProvider: React.FC = () => {
  const { t } = useTranslation();
  const [showDetails, setShowDetails] = useState({
    show: false,
    path: '',
    type: '',
    title: '',
    privderType: '',
  });
  useEffect(() => {
    if (showDetails.title) {
      const fetchProviderPath = async (name: string): Promise<void> => {
        try {
          const providers = await getRuntimeConfig();
          const provider = providers?.['proxy-providers']?.[name] as ProxyProviderConfig;
          if (provider) {
            setShowDetails(prev => ({
              ...prev,
              show: true,
              path: provider.path || `proxies/${getHash(provider.url || '')}`,
            }));
          }
        } catch {
          setShowDetails(prev => ({ ...prev, path: '' }));
        }
      };
      fetchProviderPath(showDetails.title);
    }
  }, [showDetails.title]);

  const { data, mutate } = useSWR('mihomoProxyProviders', mihomoProxyProviders, {
    errorRetryInterval: 200,
    errorRetryCount: 10,
  });

  useEffect(() => {
    const unlisten = listen('core-started', () => {
      mutate();
    });
    return (): void => {
      unlisten.then(f => f());
    };
  }, [mutate]);

  const providers = useMemo(() => {
    if (!data) return [];
    return Object.values(data.providers)
      .filter(provider => provider.vehicleType !== 'Compatible')
      .sort((a, b) => {
        const order: Record<string, number> = { File: 1, Inline: 2, HTTP: 3 };
        return (order[a.vehicleType] || 4) - (order[b.vehicleType] || 4);
      });
  }, [data]);
  const [updating, setUpdating] = useState(Array(providers.length).fill(false));

  const onUpdate = async (name: string, index: number): Promise<void> => {
    setUpdating(prev => {
      prev[index] = true;
      return [...prev];
    });
    try {
      await mihomoUpdateProxyProviders(name);
      mutate();
    } catch (e) {
      toast.error(t('resources.updateFailed', { name, error: String(e) }));
    } finally {
      setUpdating(prev => {
        prev[index] = false;
        return [...prev];
      });
    }
  };

  if (!providers.length) {
    return null;
  }

  return (
    <SettingCard>
      {showDetails.show && (
        <Viewer
          path={showDetails.path}
          type={showDetails.type}
          title={showDetails.title}
          privderType={showDetails.privderType}
          onClose={() => setShowDetails({ show: false, path: '', type: '', title: '', privderType: '' })}
        />
      )}
      <SettingItem title={t('resources.proxyProvider')} divider>
        <Button
          size='sm'
          onClick={() => {
            providers.forEach((provider, index) => {
              onUpdate(provider.name, index);
            });
          }}
        >
          {t('resources.updateAll')}
        </Button>
      </SettingItem>
      {providers.map((provider, index) => (
        <Fragment key={provider.name}>
          <SettingItem
            title={provider.name}
            actions={<Badge className='ml-2'>{provider.proxies?.length || 0}</Badge>}
            divider={!provider.subscriptionInfo && index !== providers.length - 1}
          >
            <div className='flex h-8 leading-8 text-foreground-500'>
              <div>{dayjs(provider.updatedAt).fromNow()}</div>
              <Button
                title={provider.vehicleType === 'File' ? t('resources.edit') : t('resources.view')}
                className='ml-2'
                size='icon-sm'
                variant='ghost'
                onClick={() => {
                  setShowDetails({
                    show: false,
                    privderType: 'proxy-providers',
                    path: provider.name,
                    type: provider.vehicleType,
                    title: provider.name,
                  });
                }}
              >
                {provider.vehicleType === 'File' ? (
                  <FilePenLine className={`text-lg`} />
                ) : (
                  <FileText className={`text-lg`} />
                )}
              </Button>
              <Button
                title={t('common.update')}
                className='ml-2'
                size='icon-sm'
                variant='ghost'
                onClick={() => {
                  onUpdate(provider.name, index);
                }}
              >
                <RefreshCcw className={`text-lg ${updating[index] ? 'animate-spin' : ''}`} />
              </Button>
            </div>
          </SettingItem>
          {provider.subscriptionInfo && (
            <SettingItem
              divider={index !== providers.length - 1}
              title={
                <div className='text-foreground-500'>
                  {`${calcTraffic(
                    provider.subscriptionInfo.Upload + provider.subscriptionInfo.Download,
                  )} / ${calcTraffic(provider.subscriptionInfo.Total)}`}
                </div>
              }
            >
              <div className='h-8 leading-8 text-foreground-500'>
                {provider.subscriptionInfo.Expire
                  ? dayjs.unix(provider.subscriptionInfo.Expire).format('YYYY-MM-DD')
                  : t('profile.longTermValid')}
              </div>
            </SettingItem>
          )}
        </Fragment>
      ))}
    </SettingCard>
  );
};

export default ProxyProvider;
