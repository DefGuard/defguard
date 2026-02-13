import './style.scss';
import {
  autoUpdate,
  offset,
  shift,
  useClick,
  useDismiss,
  useFloating,
  useInteractions,
} from '@floating-ui/react';
import { useSuspenseQuery } from '@tanstack/react-query';
import { Suspense, useState } from 'react';
import type { LicenseInfo } from '../../../../api/types';
import { Icon, IconKind } from '../../../../defguard-ui/components/Icon';
import { isPresent } from '../../../../defguard-ui/utils/isPresent';
import { useAuth } from '../../../../hooks/useAuth';
import { getLicenseInfoQueryOptions } from '../../../../query';
import { TopBarElementSkeleton } from '../../TopBarElementSkeleton';
import { TopBarLicenseProgress } from './components/TopBarLicenseProgress';
import { TopBarLicenseFloating } from './TopBarLicenseFloating';

export const TopBarLicense = () => {
  const isAdmin = useAuth((s) => s.isAdmin);
  if (!isAdmin) return null;

  return (
    <Suspense fallback={<TopBarElementSkeleton />}>
      <Content />
    </Suspense>
  );
};

const Content = () => {
  const [isOpen, setOpen] = useState(false);
  const { data: licenseInfo } = useSuspenseQuery(getLicenseInfoQueryOptions);

  const { context, refs, floatingStyles } = useFloating({
    placement: 'bottom',
    whileElementsMounted: autoUpdate,
    open: isOpen,
    onOpenChange: setOpen,
    middleware: [offset(12), shift()],
  });

  const click = useClick(context, {
    toggle: true,
  });

  const dismiss = useDismiss(context, {
    ancestorScroll: true,
    escapeKey: true,
    outsidePress: (event) => !(event.target as HTMLElement).closest('.menu'),
  });

  const { getReferenceProps, getFloatingProps } = useInteractions([click, dismiss]);

  return (
    <>
      <div id="top-bar-license-info" ref={refs.setReference} {...getReferenceProps()}>
        {licenseInfo === null && <OpenSource />}
        {isPresent(licenseInfo) && <LicenseCompactDisplay license={licenseInfo} />}
      </div>
      {isOpen && (
        <TopBarLicenseFloating
          ref={refs.setFloating}
          {...getFloatingProps({ style: floatingStyles })}
          license={licenseInfo}
        />
      )}
    </>
  );
};

const LicenseCompactDisplay = ({ license }: { license: LicenseInfo }) => {
  return (
    <div className="license-compact-display">
      <TopBarLicenseProgress
        icon={IconKind.Users}
        value={license.limits.users.current}
        maxValue={license.limits.users.limit}
      />
      <TopBarLicenseProgress
        icon={IconKind.LocationTracking}
        value={license.limits.locations.current}
        maxValue={license.limits.locations.limit}
      />
    </div>
  );
};

const OpenSource = () => {
  return (
    <div className="open-source">
      <Icon icon={IconKind.Config} />
      <p>Open Source Mode</p>
    </div>
  );
};
