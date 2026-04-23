import { listen } from '@tauri-apps/api/event';
import dayjs from 'dayjs';
import { FilePenLine, FileText, RefreshCcw } from 'lucide-react';
import { Fragment, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import useSWR from 'swr';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { getHash } from '@/utils/hash';
import { getControledMihomoConfig, mihomoRuleProviders, mihomoUpdateRuleProviders } from '@/utils/ipc';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';
import Viewer from './viewer';

const RuleProvider: React.FC = () => {
  const { t } = useTranslation();
  const [showDetails, setShowDetails] = useState({
    show: false,
    path: '',
    type: '',
    title: '',
    format: '',
    privderType: '',
    behavior: '',
  });
  useEffect(() => {
    if (showDetails.title) {
      const fetchProviderPath = async (name: string): Promise<void> => {
        try {
          const providers = await getControledMihomoConfig();
          const provider = providers['rule-providers']?.[name];
          if (provider) {
            setShowDetails(prev => ({
              ...prev,
              show: true,
              path: provider?.path || `rules/${getHash(provider?.url || '')}`,
              behavior: provider?.behavior || 'domain',
            }));
          }
        } catch {
          setShowDetails(prev => ({ ...prev, path: '', behavior: '' }));
        }
      };
      fetchProviderPath(showDetails.title);
    }
  }, [showDetails.title]);

  const { data, mutate } = useSWR('mihomoRuleProviders', mihomoRuleProviders, {
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
    return Object.values(data.providers).sort((a, b) => {
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
      await mihomoUpdateRuleProviders(name);
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
          format={showDetails.format}
          privderType={showDetails.privderType}
          behavior={showDetails.behavior}
          onClose={() =>
            setShowDetails({
              show: false,
              path: '',
              type: '',
              title: '',
              format: '',
              privderType: '',
              behavior: '',
            })
          }
        />
      )}
      <SettingItem title={t('resources.ruleProvider')} divider>
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
          <SettingItem title={provider.name} actions={<Badge className='ml-2'>{provider.ruleCount}</Badge>}>
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
                    privderType: 'rule-providers',
                    path: provider.name,
                    type: provider.vehicleType,
                    title: provider.name,
                    format: provider.format,
                    behavior: provider.behavior || 'domain',
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
          <SettingItem
            title={<div className='text-foreground-500'>{provider.format || 'InlineRule'}</div>}
            divider={index !== providers.length - 1}
          >
            <div className='h-8 leading-8 text-foreground-500'>
              {provider.vehicleType}::{provider.behavior}
            </div>
          </SettingItem>
        </Fragment>
      ))}
    </SettingCard>
  );
};

export default RuleProvider;
