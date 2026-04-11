import { CircleAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogMedia,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { useProfileConfig } from '@/hooks/use-profile-config';

const HwidLimitAlert = () => {
  const { t } = useTranslation();
  const { hwidLimitError, clearHwidLimitError } = useProfileConfig();

  return (
    <AlertDialog open={hwidLimitError !== null} onOpenChange={open => !open && clearHwidLimitError()}>
      <AlertDialogContent size='sm'>
        <AlertDialogHeader>
          <AlertDialogMedia>
            <CircleAlert className='size-8 text-destructive' />
          </AlertDialogMedia>
          <AlertDialogTitle>{t('pages.profiles.hwidLimitTitle')}</AlertDialogTitle>
          <AlertDialogDescription>{t('pages.profiles.hwidLimitDescription')}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={clearHwidLimitError}>{t('common.close')}</AlertDialogCancel>
          {hwidLimitError && (
            <AlertDialogAction
              onClick={() => {
                open(hwidLimitError);
                clearHwidLimitError();
              }}
            >
              {t('pages.profiles.support')}
            </AlertDialogAction>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};

export default HwidLimitAlert;
