import { Story } from '@ladle/react';
import { useState } from 'react';

import { ExpandableCard } from '../ExpandableCard';

export const ExpandableCardStory: Story = () => {
  const [expanded, setExpanded] = useState(false);
  return (
    <ExpandableCard
      title="Title of card"
      expanded={expanded}
      onChange={() => setExpanded((state) => !state)}
    >
      <p>Expanded content</p>
    </ExpandableCard>
  );
};

ExpandableCardStory.storyName = 'ExpandableCard';
