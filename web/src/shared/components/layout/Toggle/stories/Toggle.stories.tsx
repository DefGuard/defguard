import { Story } from '@ladle/react';
import { useState } from 'react';

import { Toggle, ToggleOption } from '../Toggle';

const options: ToggleOption<number>[] = [
  {
    text: 'Option 1',
    value: 1,
  },
  {
    text: 'Option 2',
    value: 2,
  },
];

export const ToggleStory: Story = () => {
  const [selected, setSelected] = useState(options[0].value);
  return (
    <>
      <Toggle
        selected={selected}
        onChange={(v) => setSelected(v)}
        options={options}
      />
      <p>Selected: {selected}</p>
    </>
  );
};

ToggleStory.storyName = 'Toggle';
