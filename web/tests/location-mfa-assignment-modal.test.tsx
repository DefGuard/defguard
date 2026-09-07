import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import {
  MfaFlowAssignmentModalContent,
  type MfaFlowAssignmentTarget,
} from '../src/pages/EditLocationPage/components/LocationMfaSection/MfaFlowAssignmentModal';
import type { MfaFlowListItemResponse } from '../src/shared/api/types';
import type { SelectionOption } from '../src/shared/components/SelectionSection/type';

const flows: MfaFlowListItemResponse[] = [
  { id: 1, title: 'FirstFlow', step_count: 0, steps: [], created_at: '', updated_at: '' },
  {
    id: 2,
    title: 'SecondFlow',
    step_count: 0,
    steps: [],
    created_at: '',
    updated_at: '',
  },
  { id: 3, title: 'ThirdFlow', step_count: 0, steps: [], created_at: '', updated_at: '' },
];

const groupOptions: SelectionOption<number>[] = [
  { id: 7, label: 'FirstGroup' },
  { id: 8, label: 'SecondGroup' },
];

type RenderOptions = {
  target?: MfaFlowAssignmentTarget;
  groups?: boolean;
  onGroupStep?: boolean;
};

const renderContent = ({
  target = {},
  groups = false,
  onGroupStep = false,
}: RenderOptions = {}) => {
  const onSubmit = vi.fn();
  const onClose = vi.fn();
  const setOnGroupStep = vi.fn();
  render(
    <MfaFlowAssignmentModalContent
      target={target}
      flows={flows}
      groupOptions={groups ? groupOptions : undefined}
      onGroupStep={onGroupStep}
      setOnGroupStep={setOnGroupStep}
      onClose={onClose}
      onSubmit={onSubmit}
    />,
  );
  return { onSubmit, onClose, setOnGroupStep };
};

const clickButton = (name: string) =>
  userEvent.click(screen.getByRole('button', { name }));

describe('MfaFlowAssignmentModalContent', () => {
  it('starts from the target flow, so a reopened editor submits it unchanged', async () => {
    const { onSubmit, onClose } = renderContent({ target: { flowId: 2 } });

    await clickButton('Submit');

    expect(onSubmit).toHaveBeenCalledWith(2, []);
    expect(onClose).toHaveBeenCalled();
  });

  it('submits a newly picked flow', async () => {
    const { onSubmit } = renderContent({ target: { flowId: 2 } });

    await userEvent.click(screen.getByText('ThirdFlow'));
    await clickButton('Submit');

    expect(onSubmit).toHaveBeenCalledWith(3, []);
  });

  it('keeps the selected flow when its radio item is clicked again', async () => {
    const { onSubmit } = renderContent({ target: { flowId: 2 } });

    await userEvent.click(screen.getByText('SecondFlow'));
    await clickButton('Submit');

    expect(onSubmit).toHaveBeenCalledWith(2, []);
  });

  it('blocks submitting without a flow', () => {
    renderContent();

    expect(screen.getByRole('button', { name: 'Submit' })).toBeDisabled();
  });

  it('sends an override to the group step instead of submitting', async () => {
    const { setOnGroupStep, onSubmit } = renderContent({
      target: { flowId: 2, groupIds: [] },
      groups: true,
    });

    expect(screen.queryByRole('button', { name: 'Submit' })).not.toBeInTheDocument();
    await clickButton('Continue');

    expect(setOnGroupStep).toHaveBeenCalledWith(true);
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('requires at least one group on an override', async () => {
    const { onSubmit, onClose } = renderContent({
      target: { flowId: 2, groupIds: [] },
      groups: true,
      onGroupStep: true,
    });

    await clickButton('Submit');

    expect(screen.getByText('Please select at least one group')).toBeInTheDocument();
    expect(onSubmit).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('submits the picked groups on an override', async () => {
    const { onSubmit } = renderContent({
      target: { flowId: 2, groupIds: [] },
      groups: true,
      onGroupStep: true,
    });

    await userEvent.click(screen.getByText('SecondGroup'));
    await clickButton('Submit');

    expect(onSubmit).toHaveBeenCalledWith(2, [8]);
  });

  it('starts from the target groups on a reopened override', async () => {
    const { onSubmit } = renderContent({
      target: { flowId: 2, groupIds: [7] },
      groups: true,
      onGroupStep: true,
    });

    await clickButton('Submit');

    expect(onSubmit).toHaveBeenCalledWith(2, [7]);
  });
});
