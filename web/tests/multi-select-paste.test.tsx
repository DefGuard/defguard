import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MultiSelect } from '../src/shared/defguard-ui/components/MultiSelect/MultiSelect';

vi.mock('@chenglou/pretext', () => ({
  prepareWithSegments: () => null,
  measureNaturalWidth: () => 0,
}));

const validateValue = (values: string[]) => !values.includes('10.0.0.256');

describe('MultiSelect paste', () => {
  it('keeps a rejected pasted list editable and splits it into chips once corrected', () => {
    const onChange = vi.fn();
    render(
      <MultiSelect
        value={[]}
        onChange={onChange}
        validateValue={validateValue}
        testId="ips"
      />,
    );
    const input = screen.getByRole('textbox') as HTMLInputElement;

    fireEvent.paste(input, {
      clipboardData: { getData: () => '10.0.0.1,10.0.0.256' },
    });

    expect(input.value).toBe('10.0.0.1,10.0.0.256');
    expect(onChange).not.toHaveBeenCalled();

    fireEvent.change(input, { target: { value: '10.0.0.1,10.0.0.2' } });
    fireEvent.keyDown(input, { key: 'Enter' });

    expect(onChange).toHaveBeenCalledWith(['10.0.0.1', '10.0.0.2']);
  });

  it('commits a valid pasted list as separate chips', () => {
    const onChange = vi.fn();
    render(<MultiSelect value={[]} onChange={onChange} validateValue={validateValue} />);
    const input = screen.getByRole('textbox') as HTMLInputElement;

    fireEvent.paste(input, {
      clipboardData: { getData: () => '10.0.0.1, 10.0.0.2' },
    });

    expect(onChange).toHaveBeenCalledWith(['10.0.0.1', '10.0.0.2']);
    expect(input.value).toBe('');
  });
});
