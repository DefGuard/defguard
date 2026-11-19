import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MultiSelect } from '../src/shared/defguard-ui/components/MultiSelect/MultiSelect';

vi.mock('@chenglou/pretext', () => ({
  prepareWithSegments: () => ({}),
  measureNaturalWidth: () => 0,
}));

const typeDraft = (text: string, selectionStart: number, selectionEnd: number) => {
  const input = screen.getByTestId<HTMLInputElement>('multi-select');
  fireEvent.change(input, { target: { value: text } });
  input.setSelectionRange(selectionStart, selectionEnd);
  return input;
};

const paste = (text: string) =>
  fireEvent.paste(screen.getByTestId('multi-select'), {
    clipboardData: { getData: () => text },
  });

describe('MultiSelect paste', () => {
  it('splits a comma-separated list into separate chips', () => {
    const onChange = vi.fn();
    render(<MultiSelect testId="multi-select" value={['1.1.1.1']} onChange={onChange} />);

    const notPrevented = paste('10.0.0.0/8, 192.168.1.1,,2000::/3');

    expect(notPrevented).toBe(false);
    expect(onChange).toHaveBeenCalledWith([
      '1.1.1.1',
      '10.0.0.0/8',
      '192.168.1.1',
      '2000::/3',
    ]);
  });

  it('leaves a single value to the default paste behavior', () => {
    const onChange = vi.fn();
    render(<MultiSelect testId="multi-select" value={[]} onChange={onChange} />);

    const notPrevented = paste('10.0.0.0/8');

    expect(notPrevented).toBe(true);
    expect(onChange).not.toHaveBeenCalled();
  });

  it('replaces a fully selected draft with the pasted chips', () => {
    const onChange = vi.fn();
    render(<MultiSelect testId="multi-select" value={[]} onChange={onChange} />);

    const input = typeDraft('10.0.0.1', 0, 8);
    paste('10.0.0.2,10.0.0.3');
    fireEvent.blur(input);

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(['10.0.0.2', '10.0.0.3']);
    expect(input).toHaveValue('');
  });

  it('keeps the unselected part of a partially selected draft', () => {
    const onChange = vi.fn();
    render(<MultiSelect testId="multi-select" value={[]} onChange={onChange} />);

    const input = typeDraft('10.0.0.1,10.0.0.9', 9, 17);
    paste('10.0.0.2,10.0.0.3');
    fireEvent.blur(input);

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith(['10.0.0.1', '10.0.0.2', '10.0.0.3']);
    expect(input).toHaveValue('');
  });

  it('adds nothing when the pasted values fail validation', () => {
    const onChange = vi.fn();
    const validateValue = vi.fn(() => false);
    render(
      <MultiSelect
        testId="multi-select"
        value={[]}
        onChange={onChange}
        validateValue={validateValue}
      />,
    );

    paste('10.0.0.0/8,invalid');

    expect(validateValue).toHaveBeenCalledWith(['10.0.0.0/8', 'invalid']);
    expect(onChange).not.toHaveBeenCalled();
  });
});
