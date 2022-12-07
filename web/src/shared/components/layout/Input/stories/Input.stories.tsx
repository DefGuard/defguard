import './style.scss';

import { Story } from '@ladle/react';
import { useEffect, useState } from 'react';

import { Input } from '../Input';

interface Props {
  value: string;
  disabled: boolean;
  outerLabel: string;
  disableOuterLabelColon: boolean;
}

export const InputStory: Story<Props> = ({ value, ...rest }) => {
  const [inputValue, setInputValue] = useState(value);

  useEffect(() => {
    setInputValue(value);
  }, [value]);

  return (
    <Input
      value={inputValue}
      onChange={(e) => setInputValue(e.target.value)}
      {...rest}
    />
  );
};

InputStory.storyName = 'Input';
InputStory.args = {
  value: 'Test value',
  outerLabel: 'Test label',
  disabled: false,
  disableOuterLabelColon: false,
};
