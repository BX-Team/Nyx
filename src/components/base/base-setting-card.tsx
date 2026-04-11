import type React from 'react';
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '@/components/ui/accordion';
import { Card, CardContent } from '@/components/ui/card';

interface Props {
  title?: string;
  children?: React.ReactNode;
  className?: string;
}

const SettingCard: React.FC<Props> = ({ title, children, className }) => {
  return !title ? (
    <Card className={`${className ?? ''} mx-2 mb-2`}>
      <CardContent>{children}</CardContent>
    </Card>
  ) : (
    <Accordion
      className={`${className ?? ''} mx-2 mb-2 px-6 rounded-xl border text-card-foreground shadow-sm`}
      type='single'
      collapsible
    >
      <AccordionItem value={title}>
        <AccordionTrigger>{title}</AccordionTrigger>
        <AccordionContent>{children}</AccordionContent>
      </AccordionItem>
    </Accordion>
  );
};

export default SettingCard;
