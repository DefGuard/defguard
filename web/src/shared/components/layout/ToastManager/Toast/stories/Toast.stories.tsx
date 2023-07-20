import './demo.scss';

import { Story } from '@ladle/react';

import { ToastOptions } from '../../../../../hooks/store/useToastStore';
import { Toast } from '../Toast';
import { ToastType } from '../types';

export const DemoStory: Story = () => {
  const data: ToastOptions[] = [
    {
      message: 'Message 1',
      type: ToastType.INFO,
      id: 1,
    },
    {
      message: 'Message 2',
      type: ToastType.ERROR,
      id: 2,
    },
    {
      message: 'Message 3',
      type: ToastType.SUCCESS,
      id: 3,
    },
    {
      message: 'Message 4',
      type: ToastType.WARNING,
      id: 4,
    },
  ];
  return (
    <div className="toasts">
      {data.map((d) => (
        <Toast data={d} key={d.id} />
      ))}
    </div>
  );
};
