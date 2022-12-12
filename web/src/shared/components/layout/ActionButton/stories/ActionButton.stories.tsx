import { Story } from '@ladle/react';

import { ActionButton, ActionButtonVariant } from '../ActionButton';

interface Props {
  variant: ActionButtonVariant;
}

export const ActionButtonStory: Story<Props> = ({ variant }) => {
  return <ActionButton variant={variant} />;
};

ActionButtonStory.storyName = 'ActionButton';
ActionButtonStory.args = {
  variant: ActionButtonVariant.COPY,
};
ActionButtonStory.argTypes = {
  variant: {
    options: [
      ActionButtonVariant.COPY,
      ActionButtonVariant.DOWNLOAD,
      ActionButtonVariant.QRCODE,
    ],
    control: { type: 'select' },
    defaultValue: ActionButtonVariant.COPY,
  },
};
