import { Story } from '@ladle/react';
import { useState } from 'react';

import { CheckBox, CheckBoxProps } from './CheckBox';

export const CheckBoxStory: Story<Pick<CheckBoxProps, 'value' | 'label'>> = ({
  label,
}) => {
  const [checked, setChecked] = useState(false);
  return (
    <CheckBox
      label={label || 'label'}
      value={Number(checked)}
      onChange={(val) => setChecked(Boolean(val))}
    />
  );
};

CheckBoxStory.storyName = 'CheckBox';
