import {
  ChevronDown,
  ChevronsDownUp,
  ChevronsRight,
  ChevronsUpDown,
  Gauge,
  LocateFixed,
  SlidersHorizontal,
} from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useLocation } from 'react-router-dom';
import { GroupedVirtuoso, type GroupedVirtuosoHandle } from 'react-virtuoso';
import BasePage from '@/components/base/base-page';
import CollapseInput from '@/components/base/collapse-input';
import ProxyItem from '@/components/proxies/proxy-item';
import ProxySettingModal from '@/components/proxies/proxy-setting-modal';
import { Avatar, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Spinner } from '@/components/ui/spinner';
import { useAppConfig } from '@/hooks/use-app-config';
import { useControledMihomoConfig } from '@/hooks/use-controled-mihomo-config';
import { useGroups } from '@/hooks/use-groups';
import { includesIgnoreCase } from '@/utils/includes';
import {
  getImageDataURL,
  mihomoChangeProxy,
  mihomoCloseAllConnections,
  mihomoProxyDelay,
  updateTrayIcon,
} from '@/utils/ipc';

const groupTypeColor: Record<string, string> = {
  Selector: 'bg-blue-500/15 text-blue-600 dark:text-blue-400',
  URLTest: 'bg-emerald-500/15 text-emerald-600 dark:text-emerald-400',
  Fallback: 'bg-amber-500/15 text-amber-600 dark:text-amber-400',
  LoadBalance: 'bg-violet-500/15 text-violet-600 dark:text-violet-400',
  Relay: 'bg-rose-500/15 text-rose-600 dark:text-rose-400',
};

const Proxies: React.FC = () => {
  const { t } = useTranslation();
  const location = useLocation();
  const fromHome = (location.state as { fromHome?: boolean })?.fromHome ?? false;
  const { controledMihomoConfig } = useControledMihomoConfig();
  const { mode = 'rule' } = controledMihomoConfig || {};
  const { groups = [], mutate } = useGroups();
  const { appConfig } = useAppConfig();
  const {
    proxyDisplayLayout = 'double',
    groupDisplayLayout = 'double',
    proxyDisplayOrder = 'default',
    autoCloseConnection = true,
    proxyCols = 'auto',
    delayTestConcurrency = 50,
    delayTestUrl = 'https://cp.cloudflare.com',
    expandProxyGroups = false,
  } = appConfig || {};
  const [cols, setCols] = useState(1);
  const [isOpen, setIsOpen] = useState(Array(groups.length).fill(expandProxyGroups));
  const [delaying, setDelaying] = useState(Array(groups.length).fill(false));
  const [searchValue, setSearchValue] = useState(Array(groups.length).fill(''));
  const [isSettingModalOpen, setIsSettingModalOpen] = useState(false);
  const virtuosoRef = useRef<GroupedVirtuosoHandle>(null);
  const { groupCounts, allProxies } = useMemo(() => {
    const groupCounts: number[] = [];
    const allProxies: (ControllerProxiesDetail | ControllerGroupDetail)[][] = [];
    if (groups.length !== searchValue.length) setSearchValue(Array(groups.length).fill(''));
    groups.forEach((group, index) => {
      if (isOpen[index]) {
        let groupProxies = group.all.filter(proxy => proxy && includesIgnoreCase(proxy.name, searchValue[index]));
        const count = Math.floor(groupProxies.length / cols);
        groupCounts.push(groupProxies.length % cols === 0 ? count : count + 1);
        if (proxyDisplayOrder === 'delay') {
          groupProxies = groupProxies.sort((a, b) => {
            if ((a.history?.length ?? 0) === 0) return -1;
            if ((b.history?.length ?? 0) === 0) return 1;
            if (a.history[a.history.length - 1].delay === 0) return 1;
            if (b.history[b.history.length - 1].delay === 0) return -1;
            return a.history[a.history.length - 1].delay - b.history[b.history.length - 1].delay;
          });
        }
        if (proxyDisplayOrder === 'name') {
          groupProxies = groupProxies.sort((a, b) => a.name.localeCompare(b.name));
        }
        allProxies.push(groupProxies);
      } else {
        groupCounts.push(0);
        allProxies.push([]);
      }
    });
    return { groupCounts, allProxies };
  }, [groups, isOpen, proxyDisplayOrder, cols, searchValue]);

  const allExpanded = useMemo(() => {
    return groups.length > 0 && isOpen.every(Boolean);
  }, [groups, isOpen]);

  const onChangeProxy = useCallback(
    async (group: string, proxy: string): Promise<void> => {
      await mihomoChangeProxy(group, proxy);
      if (autoCloseConnection) {
        await mihomoCloseAllConnections();
      }
      mutate();
      updateTrayIcon();
      void mihomoProxyDelay(proxy, delayTestUrl).catch(() => {});
    },
    [autoCloseConnection, delayTestUrl, mutate],
  );

  const onProxyDelay = useCallback(
    async (proxy: string, url?: string): Promise<ControllerProxiesDelay> => {
      return await mihomoProxyDelay(proxy, url || delayTestUrl);
    },
    [delayTestUrl],
  );

  const onGroupDelay = useCallback(
    async (index: number): Promise<void> => {
      if (allProxies[index].length === 0) {
        setIsOpen(prev => {
          const newOpen = [...prev];
          newOpen[index] = true;
          return newOpen;
        });
      }
      setDelaying(prev => {
        const newDelaying = [...prev];
        newDelaying[index] = true;
        return newDelaying;
      });
      const result: Promise<void>[] = [];
      const runningList: Promise<void>[] = [];
      for (const proxy of allProxies[index]) {
        const promise = Promise.resolve().then(async () => {
          try {
            await mihomoProxyDelay(proxy.name, delayTestUrl || groups[index].testUrl);
          } catch {
          } finally {
            mutate();
          }
        });
        result.push(promise);
        const running = promise.then(() => {
          runningList.splice(runningList.indexOf(running), 1);
        });
        runningList.push(running);
        if (runningList.length >= (delayTestConcurrency || 50)) {
          await Promise.race(runningList);
        }
      }
      await Promise.all(result);
      setDelaying(prev => {
        const newDelaying = [...prev];
        newDelaying[index] = false;
        return newDelaying;
      });
    },
    [allProxies, groups, delayTestConcurrency, delayTestUrl, mutate],
  );

  const calcCols = useCallback((): number => {
    if (window.matchMedia('(min-width: 1536px)').matches) {
      return 5;
    } else if (window.matchMedia('(min-width: 1280px)').matches) {
      return 4;
    } else if (window.matchMedia('(min-width: 1024px)').matches) {
      return 3;
    } else {
      return 2;
    }
  }, []);

  const toggleOpen = useCallback((index: number) => {
    setIsOpen(prev => {
      const newOpen = [...prev];
      newOpen[index] = !prev[index];
      return newOpen;
    });
  }, []);

  const toggleAll = useCallback(() => {
    setIsOpen(prev => {
      const shouldExpand = !prev.every(Boolean);
      return Array(prev.length).fill(shouldExpand);
    });
  }, []);

  const updateSearchValue = useCallback((index: number, value: string) => {
    setSearchValue(prev => {
      const newSearchValue = [...prev];
      newSearchValue[index] = value;
      return newSearchValue;
    });
  }, []);

  const scrollToCurrentProxy = useCallback(
    (index: number) => {
      if (!isOpen[index]) {
        setIsOpen(prev => {
          const newOpen = [...prev];
          newOpen[index] = true;
          return newOpen;
        });
      }
      let i = 0;
      for (let j = 0; j < index; j++) {
        i += groupCounts[j];
      }
      i += Math.floor(allProxies[index].findIndex(proxy => proxy.name === groups[index].now) / cols);
      virtuosoRef.current?.scrollToIndex({
        index: Math.floor(i),
        align: 'start',
      });
    },
    [isOpen, groupCounts, allProxies, groups, cols],
  );

  useEffect(() => {
    if (proxyCols !== 'auto') {
      setCols(parseInt(proxyCols, 10));
      return;
    }
    setCols(calcCols());
    const handleResize = (): void => {
      setCols(calcCols());
    };
    window.addEventListener('resize', handleResize);
    return (): void => {
      window.removeEventListener('resize', handleResize);
    };
  }, [proxyCols, calcCols]);

  const groupContent = useCallback(
    (index: number) => {
      if (groups[index]?.icon?.startsWith('http') && !localStorage.getItem(groups[index].icon)) {
        getImageDataURL(groups[index].icon).then(dataURL => {
          localStorage.setItem(groups[index].icon, dataURL);
          mutate();
        });
      }
      const group = groups[index];
      if (!group) return <div>Never See This</div>;

      const typeColorClass = groupTypeColor[group.type] || 'bg-muted text-muted-foreground';

      return (
        <div className={`w-full ${!isOpen[index] ? 'pb-2' : ''} px-2`}>
          <Card
            className='w-full backdrop-blur-3xl cursor-pointer py-0 transition-all duration-200 hover:bg-accent/50 hover:shadow-sm'
            role='button'
            tabIndex={0}
            onClick={() => toggleOpen(index)}
            onKeyDown={event => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                toggleOpen(index);
              }
            }}
          >
            <CardContent className='w-full px-4 py-3'>
              <div className='flex justify-between items-center'>
                {/* Left side: icon + name + meta */}
                <div className='flex items-center gap-3 min-w-0 flex-1'>
                  {group.icon ? (
                    <Avatar className='bg-transparent rounded-md shrink-0 size-9'>
                      <AvatarImage
                        src={
                          group.icon.startsWith('<svg')
                            ? `data:image/svg+xml;utf8,${group.icon}`
                            : localStorage.getItem(group.icon) || group.icon
                        }
                      />
                    </Avatar>
                  ) : null}
                  <div
                    className={`flex ${groupDisplayLayout === 'double' ? 'flex-col gap-0.5' : 'items-center gap-2'} min-w-0`}
                  >
                    <span className='flag-emoji text-sm font-medium truncate leading-tight'>{group.name}</span>
                    {groupDisplayLayout !== 'hidden' && (
                      <div className='flex items-center gap-1.5 text-xs text-muted-foreground leading-tight min-w-0'>
                        <Badge
                          variant='ghost'
                          className={`text-[10px] px-1.5 py-0 h-4 rounded-md font-medium shrink-0 ${typeColorClass}`}
                        >
                          {group.type}
                        </Badge>
                        <span className='flag-emoji truncate'>{group.now}</span>
                      </div>
                    )}
                  </div>
                </div>

                {/* Right side: actions */}
                <div className='flex items-center gap-0.5 shrink-0'>
                  <div className='flex items-center' onClick={e => e.stopPropagation()}>
                    <CollapseInput value={searchValue[index]} onValueChange={v => updateSearchValue(index, v)} />
                    <Button
                      title={t('sider.locateCurrentNode')}
                      variant='ghost'
                      size='icon-sm'
                      onClick={() => scrollToCurrentProxy(index)}
                    >
                      <LocateFixed className='text-base' />
                    </Button>
                    <Button
                      title={t('sider.delayTest')}
                      variant='ghost'
                      size='icon-sm'
                      disabled={delaying[index]}
                      aria-busy={delaying[index]}
                      onClick={() => onGroupDelay(index)}
                    >
                      {delaying[index] ? <Spinner className='size-4' /> : <Gauge className='text-base' />}
                    </Button>
                  </div>
                  <ChevronDown
                    className={`transition-transform duration-200 ml-1 size-5 ${isOpen[index] ? 'rotate-180' : ''}`}
                  />
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      );
    },
    [
      groups,
      isOpen,
      groupDisplayLayout,
      searchValue,
      delaying,
      toggleOpen,
      updateSearchValue,
      scrollToCurrentProxy,
      onGroupDelay,
      mutate,
      t,
    ],
  );

  const itemContent = useCallback(
    (index: number, groupIndex: number) => {
      let innerIndex = index;
      groupCounts.slice(0, groupIndex).forEach(count => {
        innerIndex -= count;
      });
      return allProxies[groupIndex] ? (
        <div
          style={proxyCols !== 'auto' ? { gridTemplateColumns: `repeat(${proxyCols}, minmax(0, 1fr))` } : {}}
          className={`grid ${proxyCols === 'auto' ? 'sm:grid-cols-2 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5' : ''} ${innerIndex === groupCounts[groupIndex] - 1 ? 'pb-2' : ''} gap-2 pt-2 mx-2`}
        >
          {Array.from({ length: cols }).map((_, i) => {
            if (!allProxies[groupIndex][innerIndex * cols + i]) return null;
            return (
              <ProxyItem
                key={allProxies[groupIndex][innerIndex * cols + i].name}
                mutateProxies={mutate}
                onProxyDelay={onProxyDelay}
                onSelect={onChangeProxy}
                proxy={allProxies[groupIndex][innerIndex * cols + i]}
                group={groups[groupIndex]}
                proxyDisplayLayout={proxyDisplayLayout}
                selected={allProxies[groupIndex][innerIndex * cols + i]?.name === groups[groupIndex].now}
              />
            );
          })}
        </div>
      ) : (
        <div>Never See This</div>
      );
    },
    [groupCounts, allProxies, proxyCols, cols, mutate, onProxyDelay, onChangeProxy, groups, proxyDisplayLayout],
  );

  return (
    <BasePage
      title={t('pages.proxies.title')}
      showBackButton={fromHome}
      header={
        <>
          <Button
            size='icon-sm'
            variant='ghost'
            className='app-nodrag'
            title={allExpanded ? t('pages.proxies.collapseAll') : t('pages.proxies.expandAll')}
            onClick={toggleAll}
          >
            {allExpanded ? <ChevronsDownUp className='text-lg' /> : <ChevronsUpDown className='text-lg' />}
          </Button>
          <Button
            size='icon-sm'
            variant='ghost'
            className='app-nodrag'
            title={t('pages.proxies.proxyGroupSettings')}
            onClick={() => setIsSettingModalOpen(true)}
          >
            <SlidersHorizontal className='text-lg' />
          </Button>
        </>
      }
    >
      {isSettingModalOpen && <ProxySettingModal onClose={() => setIsSettingModalOpen(false)} />}
      {mode === 'direct' ? (
        <div className='h-full w-full flex justify-center items-center'>
          <div className='flex flex-col items-center gap-3'>
            <div className='rounded-full bg-muted p-6'>
              <ChevronsRight className='text-muted-foreground text-5xl' />
            </div>
            <h2 className='text-muted-foreground text-lg font-medium'>{t('sider.directMode')}</h2>
          </div>
        </div>
      ) : (
        <div className='h-[calc(100vh-58px)]'>
          <GroupedVirtuoso
            ref={virtuosoRef}
            groupCounts={groupCounts}
            groupContent={groupContent}
            itemContent={itemContent}
          />
        </div>
      )}
    </BasePage>
  );
};

export default Proxies;
