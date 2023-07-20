import './demo.scss';

import { Story } from '@ladle/react';

import { MessageBox } from '../MessageBox';
import { MessageBoxType } from '../types';

const veryLongMessage =
  // eslint-disable-next-line max-len
  'Suspendisse placerat massa et urna volutpat feugiat. Pellentesque pretium eget ipsum eget fringilla. Etiam vitae facilisis magna. Ut neque quam, luctus ac lorem at, vulputate facilisis enim. In varius ligula a iaculis volutpat. Duis ultricies felis sit amet lorem faucibus, nec cursus libero dapibus. Sed iaculis, nulla non suscipit ultricies, est massa lacinia dui, vehicula mattis tellus ex sed nisi. Nunc commodo velit vitae auctor semper. Morbi libero diam, sodales quis mi nec, bibendum blandit lorem';

export const DemoStory: Story = () => {
  const message = 'Your passwod has been changed.';
  return (
    <div className="demo-message-box">
      <MessageBox type={MessageBoxType.INFO} message={message} dismissId="dismiss-1" />
      <MessageBox type={MessageBoxType.ERROR} message={message} />
      <MessageBox type={MessageBoxType.SUCCESS} message={message} />
      <MessageBox type={MessageBoxType.WARNING} message={message} />

      <MessageBox type={MessageBoxType.INFO} message={veryLongMessage} />
      <MessageBox type={MessageBoxType.ERROR} message={veryLongMessage} />
      <MessageBox type={MessageBoxType.SUCCESS} message={veryLongMessage} />
      <MessageBox type={MessageBoxType.WARNING} message={veryLongMessage} />
    </div>
  );
};
