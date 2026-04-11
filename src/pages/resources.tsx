import { useTranslation } from 'react-i18next';
import BasePage from '@/components/base/base-page';
import GeoData from '@/components/resources/geo-data';
import ProxyProvider from '@/components/resources/proxy-provider';
import RuleProvider from '@/components/resources/rule-provider';

const Resources: React.FC = () => {
  const { t } = useTranslation();
  return (
    <BasePage title={t('pages.resources.title')}>
      <GeoData />
      <ProxyProvider />
      <RuleProvider />
    </BasePage>
  );
};

export default Resources;
