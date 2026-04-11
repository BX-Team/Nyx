import { Search } from 'lucide-react';
import type React from 'react';
import { useRef, useState } from 'react';
import { InputGroup, InputGroupAddon, InputGroupButton, InputGroupInput } from '@/components/ui/input-group';

interface CollapseInputProps {
  value: string;
  onValueChange: (value: string) => void;
}

const CollapseInput: React.FC<CollapseInputProps> = ({ value, onValueChange }) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const [isExpanded, setIsExpanded] = useState(false);

  if (!isExpanded) {
    return (
      <InputGroupButton
        onClick={e => {
          e.stopPropagation();
          setIsExpanded(true);
          setTimeout(() => {
            inputRef.current?.focus();
          }, 0);
        }}
      >
        <Search />
      </InputGroupButton>
    );
  }

  return (
    <InputGroup>
      <InputGroupInput
        ref={inputRef}
        value={value}
        onChange={e => onValueChange?.(e.target.value)}
        onClick={e => {
          e.stopPropagation();
        }}
      />
      <InputGroupAddon align='inline-end'>
        <InputGroupButton
          onClick={e => {
            e.stopPropagation();
            setIsExpanded(false);
            onValueChange?.('');
          }}
        >
          <Search />
        </InputGroupButton>
      </InputGroupAddon>
    </InputGroup>
  );
};

export default CollapseInput;
