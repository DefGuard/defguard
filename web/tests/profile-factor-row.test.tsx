import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { FactorRow } from '../src/pages/user-profile/UserProfilePage/tabs/ProfileDetailsTab/components/ProfileAuthCard/components/FactorRow/FactorRow';

describe('FactorRow with SMTP not configured', () => {
  it('shows a single Unavailable label and keeps the Default badge', () => {
    render(
      <FactorRow
        icon="mail"
        title="Email"
        enabled
        isDefault
        smtpDisabled
        testId="email-codes-row"
      />,
    );

    const row = within(screen.getByTestId('email-codes-row'));
    expect(row.getByText('Unavailable')).toBeInTheDocument();
    expect(row.getByText('Default method')).toBeInTheDocument();
    expect(row.queryByText('Not configured')).not.toBeInTheDocument();
    expect(row.queryByText('Enabled')).not.toBeInTheDocument();
  });
});
