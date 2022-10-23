import { Story } from '@ladle/react';

import Button, { ButtonSize, ButtonStyleVariant } from '../Button/Button';

export const ButtonStory: Story<{
  text: string;
  size: ButtonSize;
  styleVariant: ButtonStyleVariant;
  loading: boolean;
  disabled: boolean;
}> = ({ text, size, styleVariant, disabled, loading }) => (
  <Button
    text={text}
    size={size}
    styleVariant={styleVariant}
    loading={loading}
    disabled={disabled}
  />
);
ButtonStory.storyName = 'Button';
ButtonStory.args = {
  text: 'button text',
  loading: false,
  disabled: false,
  size: ButtonSize.BIG,
  styleVariant: ButtonStyleVariant.STANDARD,
};
ButtonStory.argTypes = {
  size: {
    options: Object.values(ButtonSize),
    defaultValue: ButtonSize.BIG,
    control: {
      type: 'select',
    },
  },
  styleVariant: {
    options: Object.values(ButtonStyleVariant),
    defaultValue: ButtonStyleVariant.STANDARD,
    control: {
      type: 'select',
    },
  },
};
