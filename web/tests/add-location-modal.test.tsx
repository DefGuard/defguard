import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AddLocationModalContent } from '../src/pages/LocationsPage/modals/AddLocationModal/AddLocationModal';
import { LicenseFeature, type LicenseInfo } from '../src/shared/api/types';

// useNavigate would otherwise require a RouterProvider; the gating logic under test doesn't.
vi.mock('@tanstack/react-router', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@tanstack/react-router')>()),
  useNavigate: () => vi.fn(),
}));

const makeLicense = (overrides: Partial<LicenseInfo> = {}): LicenseInfo => ({
  subscription: false,
  valid_until: null,
  expired: false,
  limits_exceeded: false,
  tier: 'Business',
  limits: null,
  features: [],
  support_type: 'Free',
  support_type_narrow: 'Free',
  ...overrides,
});

describe('AddLocationModalContent service-location gating', () => {
  const getServiceOption = () => screen.getByTestId('add-service-location');

  it('enables the service-location option when the flag is granted on a Business tier', () => {
    render(
      <AddLocationModalContent
        modalData={{
          license: makeLicense({
            tier: 'Business',
            features: [LicenseFeature.ServiceLocations],
          }),
        }}
      />,
    );

    expect(getServiceOption()).not.toHaveClass('disabled');
    expect(getServiceOption()).not.toHaveTextContent('Enterprise');
  });

  it('locks the service-location option when the flag is absent', () => {
    render(
      <AddLocationModalContent
        modalData={{ license: makeLicense({ tier: 'Business', features: [] }) }}
      />,
    );

    expect(getServiceOption()).toHaveClass('disabled');
    expect(getServiceOption()).toHaveTextContent('Enterprise');
  });
});
