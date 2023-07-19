import { Story } from '@ladle/react';

import { Input } from '../Input';

export const InputStory: Story = () => {
  return <Input />;
};

InputStory.storyName = 'Interactive';

export const DemoStory: Story = () => {
  return (
    <>
      <Input placeholder="Placeholder" label="Some label" />
      <Input placeholder="Placeholder" required />
      <Input placeholder="Placeholder" required disabled />
      <Input placeholder="Placeholder" disabled />
      <Input
        placeholder="Placeholder"
        value="SomeText"
        invalid
        errorMessage="Some error message"
        required
        disposable
      />
      <Input
        placeholder="Placeholder"
        value="SomeText"
        invalid
        errorMessage="Some error message"
        required
        disposable
        floatingErrors={{
          title: 'Some errors title',
          errorMessages: ['Some error message', 'Err2'],
        }}
      />
    </>
  );
};

DemoStory.storyName = 'Demo';
