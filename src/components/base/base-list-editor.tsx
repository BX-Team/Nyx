import { Trash2 } from 'lucide-react';
import type React from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import type { ValidationResult } from '@/utils/validate';

interface EditableListProps {
  title?: string;
  items: string[] | Record<string, string | string[]> | Array<{ key: string; value: string | string[] }>;
  onChange: (items: unknown) => void;
  placeholder?: string;
  part2Placeholder?: string;
  parse?: (item: string) => { part1: string; part2?: string };
  format?: (part1: string, part2?: string) => string;
  disableFirst?: boolean;
  divider?: boolean;
  objectMode?: 'keyValue' | 'array' | 'record';
  validate?: (part1: string, part2?: string) => boolean | ValidationResult;
  validatePart1?: (part1: string) => boolean | ValidationResult;
  validatePart2?: (part2: string) => boolean | ValidationResult;
}

const EditableList: React.FC<EditableListProps> = ({
  title,
  items = [],
  onChange,
  placeholder = '',
  part2Placeholder = '',
  parse,
  format,
  disableFirst = false,
  divider = true,
  objectMode,
  validate,
  validatePart1,
  validatePart2,
}) => {
  const { t } = useTranslation();
  const isDual = !!parse && !!format;
  const formatErrorText = t('common.formatError');

  let processedItems: Array<{ part1: string; part2?: string }> = [];

  if (objectMode === 'record' && !Array.isArray(items)) {
    processedItems = Object.entries(items).map(([key, value]) => ({
      part1: key,
      part2: Array.isArray(value) ? value.join(',') : String(value),
    }));
  } else if (objectMode === 'keyValue' && Array.isArray(items)) {
    processedItems = (items as Array<{ key: string; value: string | string[] }>).map(item => ({
      part1: item.key,
      part2: Array.isArray(item.value) ? item.value.join(',') : String(item.value),
    }));
  } else if (objectMode === 'array' && Array.isArray(items)) {
    processedItems = (items as string[]).map(value => ({ part1: value }));
  } else if (isDual && Array.isArray(items)) {
    processedItems = (items as string[]).map(it => ({ ...parse?.(it) }));
  } else if (Array.isArray(items)) {
    processedItems = (items as string[]).map(i => ({ part1: i }));
  }

  const extra = isDual || objectMode ? { part1: '', part2: '' } : { part1: '' };
  const displayed = [...processedItems, extra];

  const handleUpdate = (idx: number, part1: string, part2?: string): void => {
    const isEmpty = !part1.trim() && !part2?.trim();

    if (idx < processedItems.length && isEmpty) {
      processedItems.splice(idx, 1);
    } else if (idx === processedItems.length) {
      if (isEmpty) return;
      processedItems.push({ part1, part2 });
    } else {
      processedItems[idx] = { part1, part2 };
    }

    if (objectMode === 'array') {
      const result: string[] = processedItems.map(item => item.part1);
      onChange(result);
      return;
    }

    if (objectMode === 'record') {
      const result: Record<string, string[]> = {};
      processedItems.forEach(item => {
        if (item.part1.trim()) {
          result[item.part1] = item.part2 ? item.part2.split(',').map(s => s.trim()) : [];
        }
      });
      onChange(result);
      return;
    }

    if (objectMode === 'keyValue') {
      const result = processedItems.map(item => ({
        key: item.part1,
        value: item.part2 ? item.part2.split(',').map(s => s.trim()) : [],
      }));
      onChange(result);
      return;
    }

    if (isDual) {
      const formatted = processedItems.map(({ part1, part2 }) => format?.(part1, part2));
      onChange(formatted);
      return;
    }

    onChange(processedItems.map(item => item.part1));
  };

  return (
    <>
      <div className={`flex flex-col space-y-2 ${!title ? 'mt-2' : ''}`}>
        {title && <h4 className='text-base font-medium'>{title}</h4>}
        {displayed.map((entry, idx) => {
          const disabled = disableFirst && idx === 0;
          const isExtra = idx === processedItems.length;
          const isEmpty = !entry.part1.trim() && !entry.part2?.trim();

          const rawValidation = isExtra || isEmpty ? true : validate ? validate(entry.part1, entry.part2) : true;
          const validation: ValidationResult =
            typeof rawValidation === 'boolean'
              ? { ok: rawValidation, error: rawValidation ? undefined : formatErrorText }
              : rawValidation;

          const rawValidation1 =
            isExtra || !entry.part1.trim() ? true : validatePart1 ? validatePart1(entry.part1) : true;
          const validation1: ValidationResult =
            typeof rawValidation1 === 'boolean'
              ? { ok: rawValidation1, error: rawValidation1 ? undefined : formatErrorText }
              : rawValidation1;

          const rawValidation2 =
            isExtra || !entry.part2?.trim() ? true : validatePart2 ? validatePart2(entry.part2) : true;
          const validation2: ValidationResult =
            typeof rawValidation2 === 'boolean'
              ? { ok: rawValidation2, error: rawValidation2 ? undefined : formatErrorText }
              : rawValidation2;

          const part1Valid = validatePart1 ? validation1.ok : validation.ok;
          const part2Valid = validatePart2 ? validation2.ok : validation.ok;
          const part1Error = validatePart1 ? validation1.error : validation.error;
          const part2Error = validatePart2 ? validation2.error : validation.error;

          return (
            <div key={idx} className='flex items-center space-x-2'>
              {isDual || objectMode ? (
                <>
                  <div className='w-1/3'>
                    <Tooltip open={!part1Valid}>
                      <TooltipTrigger asChild>
                        <Input
                          className={part1Valid ? 'h-8' : 'h-8 border-red-500 ring-1 ring-red-500 rounded-lg'}
                          disabled={disabled}
                          placeholder={placeholder}
                          value={entry.part1}
                          onChange={e => handleUpdate(idx, e.target.value, entry.part2)}
                        />
                      </TooltipTrigger>
                      <TooltipContent
                        side='left'
                        sideOffset={10}
                        className='bg-destructive text-destructive-foreground'
                      >
                        {part1Error ?? formatErrorText}
                      </TooltipContent>
                    </Tooltip>
                  </div>
                  <span className='mx-1'>:</span>
                  <div className='flex-1'>
                    <Tooltip open={!part2Valid}>
                      <TooltipTrigger asChild>
                        <Input
                          className={part2Valid ? 'h-8' : 'h-8 border-red-500 ring-1 ring-red-500 rounded-lg'}
                          disabled={disabled}
                          placeholder={part2Placeholder}
                          value={entry.part2 || ''}
                          onChange={e => handleUpdate(idx, entry.part1, e.target.value)}
                        />
                      </TooltipTrigger>
                      <TooltipContent
                        side='left'
                        sideOffset={10}
                        className='bg-destructive text-destructive-foreground'
                      >
                        {part2Error ?? formatErrorText}
                      </TooltipContent>
                    </Tooltip>
                  </div>
                </>
              ) : (
                <Tooltip open={!part1Valid}>
                  <TooltipTrigger asChild>
                    <Input
                      className={part1Valid ? 'h-8' : 'h-8 border-red-500 ring-1 ring-red-500 rounded-lg'}
                      disabled={disabled}
                      placeholder={placeholder}
                      value={entry.part1}
                      onChange={e => handleUpdate(idx, e.target.value)}
                    />
                  </TooltipTrigger>
                  <TooltipContent side='left' sideOffset={10} className='bg-destructive text-destructive-foreground'>
                    {part1Error ?? formatErrorText}
                  </TooltipContent>
                </Tooltip>
              )}
              {idx < processedItems.length && !disabled && (
                <Button
                  size='sm'
                  variant='ghost'
                  className='text-amber-500 hover:text-amber-600'
                  onClick={() => handleUpdate(idx, '', '')}
                >
                  <Trash2 className='text-lg' />
                </Button>
              )}
            </div>
          );
        })}
      </div>
      {divider && <Separator className='mt-2 mb-2' />}
    </>
  );
};

export default EditableList;
