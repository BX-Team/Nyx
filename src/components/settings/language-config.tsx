import type React from 'react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { useLanguage } from '@/hooks/use-language';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';

const LanguageConfig: React.FC = () => {
  const { currentLanguage, changeLanguage, languages, t } = useLanguage();

  return (
    <SettingCard>
      <SettingItem title={t('settings.appearance.language')}>
        <Select
          value={currentLanguage}
          onValueChange={value => {
            changeLanguage(value as 'zh-CN' | 'en-US' | 'ru-RU');
          }}
        >
          <SelectTrigger size='sm' className='w-[200px]'>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {languages.map(lang => (
              <SelectItem key={lang.value} value={lang.value}>
                {lang.nativeLabel}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </SettingItem>
    </SettingCard>
  );
};

export default LanguageConfig;
