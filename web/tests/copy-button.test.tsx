import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CopyButton } from '../src/shared/components/CopyButton/CopyButton';
import { Snackbar } from '../src/shared/defguard-ui/providers/snackbar/snackbar';

const copiedText = 'Content copied to clipboard.';

const mockClipboard = (writeText: (value: string) => Promise<void>) => {
  Object.defineProperty(window, 'isSecureContext', { value: true, configurable: true });
  Object.defineProperty(navigator, 'clipboard', {
    value: { writeText: vi.fn(writeText) },
    configurable: true,
  });
};

describe('CopyButton', () => {
  let snackbarError: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    snackbarError = vi
      .spyOn(Snackbar, 'error')
      .mockImplementation(() => undefined as never);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('confirms the copy after a successful clipboard write', async () => {
    mockClipboard(() => Promise.resolve());
    render(<CopyButton value="secret" testId="copy" />);

    fireEvent.click(screen.getByTestId('copy'));

    expect(await screen.findByText(copiedText)).toBeInTheDocument();
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('secret');
    expect(snackbarError).not.toHaveBeenCalled();
  });

  it('shows an error instead of a confirmation when the clipboard write is denied', async () => {
    mockClipboard(() => Promise.reject(new DOMException('denied', 'NotAllowedError')));
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
    render(<CopyButton value="secret" testId="copy" />);

    fireEvent.click(screen.getByTestId('copy'));

    await waitFor(() => {
      expect(snackbarError).toHaveBeenCalledOnce();
    });
    expect(screen.queryByText(copiedText)).not.toBeInTheDocument();
  });
});
