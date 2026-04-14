import { closestCenter, DndContext, type DragEndEvent, PointerSensor, useSensor, useSensors } from '@dnd-kit/core';
import { SortableContext } from '@dnd-kit/sortable';
import { FileDown, Plus, RefreshCcw } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import BasePage from '@/components/base/base-page';
import EditInfoModal from '@/components/profiles/edit-info-modal';
import ProfileItem from '@/components/profiles/profile-item';
import { Button } from '@/components/ui/button';
import { useProfileConfig } from '@/hooks/use-profile-config';

const emptyItems: ProfileItem[] = [];

const Profiles: React.FC = () => {
  const { t } = useTranslation();
  const {
    profileConfig,
    setProfileConfig,
    addProfileItem,
    updateProfileItem,
    removeProfileItem,
    changeCurrentProfile,
  } = useProfileConfig();
  const { current, items } = profileConfig || {};
  const itemsArray = items ?? emptyItems;
  const [sortedItems, setSortedItems] = useState(itemsArray);
  const [updating, setUpdating] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [fileOver, setFileOver] = useState(false);
  const [showEditModal, setShowEditModal] = useState(false);
  const [editingItem, setEditingItem] = useState<ProfileItem | null>(null);
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 2,
      },
    }),
  );
  const pageRef = useRef<HTMLDivElement>(null);
  const dragCounterRef = useRef(0);
  const addProfileItemRef = useRef(addProfileItem);
  addProfileItemRef.current = addProfileItem;
  const tRef = useRef(t);
  tRef.current = t;

  const onDragEnd = async (event: DragEndEvent): Promise<void> => {
    const { active, over } = event;
    if (over) {
      if (active.id !== over.id) {
        const newOrder = sortedItems.slice();
        const activeIndex = newOrder.findIndex(item => item.id === active.id);
        const overIndex = newOrder.findIndex(item => item.id === over.id);
        newOrder.splice(activeIndex, 1);
        newOrder.splice(overIndex, 0, itemsArray[activeIndex]);
        setSortedItems(newOrder);
        await setProfileConfig({ current, items: newOrder });
      }
    }
  };

  const handleDragOver = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleDragEnter = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current++;
    if (dragCounterRef.current === 1) {
      setFileOver(true);
    }
  }, []);

  const handleDragLeave = useCallback((e: DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current--;
    if (dragCounterRef.current === 0) {
      setFileOver(false);
    }
  }, []);

  const handleDrop = useCallback(async (event: DragEvent) => {
    event.preventDefault();
    event.stopPropagation();
    dragCounterRef.current = 0;
    setFileOver(false);
    if (event.dataTransfer?.files) {
      const file = event.dataTransfer.files[0];
      if (
        file.name.endsWith('.yml') ||
        file.name.endsWith('.yaml') ||
        file.name.endsWith('.json') ||
        file.name.endsWith('.jsonc') ||
        file.name.endsWith('.json5') ||
        file.name.endsWith('.txt')
      ) {
        try {
          const content = await file.text();
          await addProfileItemRef.current({ name: file.name, type: 'local', file: content });
        } catch (e) {
          toast.error(tRef.current('pages.profiles.fileImportFailed') + e);
        }
      } else {
        toast.error(tRef.current('pages.profiles.unsupportedFileType'));
      }
    }
  }, []);

  useEffect(() => {
    const el = pageRef.current;
    if (!el) return;
    el.addEventListener('dragover', handleDragOver);
    el.addEventListener('dragenter', handleDragEnter);
    el.addEventListener('dragleave', handleDragLeave);
    el.addEventListener('drop', handleDrop);
    return (): void => {
      el.removeEventListener('dragover', handleDragOver);
      el.removeEventListener('dragenter', handleDragEnter);
      el.removeEventListener('dragleave', handleDragLeave);
      el.removeEventListener('drop', handleDrop);
    };
  }, [handleDragOver, handleDragEnter, handleDragLeave, handleDrop]);

  useEffect(() => {
    setSortedItems(itemsArray);
  }, [itemsArray]);

  const handleAddProfile = (): void => {
    const newProfile: ProfileItem = {
      id: '',
      name: '',
      type: 'remote',
      url: '',
      useProxy: false,
      autoUpdate: true,
    };
    setEditingItem(newProfile);
    setShowEditModal(true);
  };

  return (
    <BasePage
      ref={pageRef}
      title={t('pages.profiles.title')}
      header={
        <>
          <Button className='new-profile app-nodrag' variant='ghost' size='icon-sm' onClick={handleAddProfile}>
            <Plus />
          </Button>
          <Button
            size='icon-sm'
            title={t('pages.profiles.updateAll')}
            className='app-nodrag'
            variant='ghost'
            aria-label={t('pages.profiles.updateAll')}
            onClick={async () => {
              setUpdating(true);
              for (const item of itemsArray) {
                if (item.id === current) continue;
                if (item.type !== 'remote') continue;
                await addProfileItem(item);
              }
              const currentItem = itemsArray.find(item => item.id === current);
              if (currentItem && currentItem.type === 'remote') {
                await addProfileItem(currentItem);
              }
              setUpdating(false);
            }}
          >
            <RefreshCcw className={`text-lg ${updating ? 'animate-spin' : ''}`} />
          </Button>
        </>
      }
    >
      {showEditModal && editingItem && (
        <EditInfoModal
          item={editingItem}
          isCurrent={editingItem.id === current}
          updateProfileItem={async (item: ProfileItem) => {
            await addProfileItem(item);
            setShowEditModal(false);
            setEditingItem(null);
          }}
          onClose={() => {
            setShowEditModal(false);
            setEditingItem(null);
          }}
        />
      )}

      {/* File drop overlay */}
      {fileOver && (
        <div className='absolute inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm pointer-events-none'>
          <div className='flex flex-col items-center gap-3 rounded-xl border-2 border-dashed border-primary/50 bg-primary/5 px-12 py-8'>
            <FileDown className='size-10 text-primary' />
            <span className='text-sm font-medium text-primary'>{t('pages.profiles.dropFileHint')}</span>
          </div>
        </div>
      )}

      {sortedItems.length === 0 ? (
        <div className='h-full w-full flex justify-center items-center'>
          <div className='flex flex-col items-center gap-3'>
            <Button className='rounded-full w-20 h-20 hover:bg-card' variant='outline' onClick={handleAddProfile}>
              <Plus className='text-muted-foreground size-10' />
            </Button>
            <h2 className='text-muted-foreground text-lg font-medium'>{t('pages.profiles.emptyTitle')}</h2>
            <p className='text-muted-foreground/70 text-sm'>{t('pages.profiles.emptyDescription')}</p>
          </div>
        </div>
      ) : (
        <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={onDragEnd}>
          <div className='grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-2 mx-2 mb-2'>
            <SortableContext
              items={sortedItems.map(item => {
                return item.id;
              })}
            >
              {sortedItems.map(item => (
                <ProfileItem
                  key={item.id}
                  isCurrent={item.id === current}
                  addProfileItem={addProfileItem}
                  removeProfileItem={removeProfileItem}
                  updateProfileItem={updateProfileItem}
                  info={item}
                  switching={switching}
                  onClick={async () => {
                    setSwitching(true);
                    await changeCurrentProfile(item.id);
                    await new Promise(resolve => setTimeout(resolve, 500));
                    setSwitching(false);
                  }}
                />
              ))}
            </SortableContext>
          </div>
        </DndContext>
      )}
    </BasePage>
  );
};

export default Profiles;
