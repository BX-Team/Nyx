import type React from 'react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';

const RuleItem: React.FC<ControllerRulesDetail & { index: number }> = props => {
  const { type, payload, proxy, index } = props;
  return (
    <div className={`px-2 pb-2 ${index === 0 ? 'pt-2' : ''}`}>
      <Card className='gap-0 py-0'>
        <CardContent className='w-full px-3 py-2'>
          {payload && (
            <div title={payload} className='text-sm text-ellipsis whitespace-nowrap overflow-hidden mb-1'>
              {payload}
            </div>
          )}
          <div className='flex gap-1.5'>
            <Badge variant='outline' className='rounded-sm'>
              {type}
            </Badge>
            <Badge variant='outline' className='rounded-sm flag-emoji whitespace-nowrap overflow-hidden'>
              {proxy}
            </Badge>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};

export default RuleItem;
