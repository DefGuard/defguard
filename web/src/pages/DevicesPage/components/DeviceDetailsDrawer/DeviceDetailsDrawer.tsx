import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { m } from '../../../../paraglide/messages';
import type { DeviceOverview } from '../../../../shared/api/types';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { ButtonMenu } from '../../../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { DrawerModal } from '../../../../shared/defguard-ui/components/DrawerModal/DrawerModal';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { getUsersDisplayQueryOptions } from '../../../../shared/query';
import { displayDate } from '../../../../shared/utils/displayDate';
import { formatIpForDisplay } from '../../../../shared/utils/formatIpForDisplay';
import { attestationStatusText, verificationText } from '../../consts';
import { platformText, systemText } from '../../helpers';
import { useDeviceActions } from '../../hooks/useDeviceActions';
import { useEnterpriseTier } from '../../hooks/useEnterpriseTier';
import { ConnectionBadge } from '../ConnectionBadge/ConnectionBadge';
import { DeviceStateBadge } from '../DeviceStateBadge/DeviceStateBadge';
import { UnknownBadge } from '../UnknownBadge/UnknownBadge';
import { DeviceDetailsSection } from './components/DeviceDetailsSection/DeviceDetailsSection';
import type { DeviceDetailsRow } from './components/DeviceDetailsSection/types';

const lastConnectionDateFormat = 'YYYY-MM-DD HH:mm';

type Props = {
  device: DeviceOverview | null;
  onClose: () => void;
};

export const DeviceDetailsDrawer = ({ device, onClose }: Props) => {
  return (
    <DrawerModal
      isOpen={isPresent(device)}
      onClose={onClose}
      title={m.devices_details_title()}
      contentClassName="device-details-drawer"
    >
      {isPresent(device) && <DrawerContent device={device} onClose={onClose} />}
    </DrawerModal>
  );
};

type ContentProps = {
  device: DeviceOverview;
  onClose: () => void;
};

const DrawerContent = ({ device, onClose }: ContentProps) => {
  const { data: usersDisplay } = useQuery(getUsersDisplayQueryOptions);
  const { getMenuItems } = useDeviceActions();
  const isEnterprise = useEnterpriseTier();

  const generalRows = useMemo((): DeviceDetailsRow[] => {
    const { os_type: osType } = device;
    return [
      { label: m.devices_details_label_name(), value: device.name },
      ...(isPresent(device.user_id)
        ? [{ label: m.devices_col_user(), value: usersDisplay?.[device.user_id] }]
        : []),
      {
        label: m.devices_col_platform(),
        value: isPresent(osType) ? platformText(osType) : <UnknownBadge />,
      },
      {
        label: m.devices_col_system(),
        value: isPresent(osType) ? (
          systemText(osType, device.os_version)
        ) : (
          <UnknownBadge />
        ),
      },
      {
        label: m.devices_col_client_version(),
        value: device.client_version ?? <UnknownBadge />,
      },
      {
        label: m.devices_col_state(),
        value: <DeviceStateBadge state={device.state} removeBackground />,
      },
      {
        label: m.devices_col_connection(),
        value: <ConnectionBadge connected={device.connected} removeBackground />,
      },
    ];
  }, [device, usersDisplay]);

  const lastConnectionRows = useMemo(
    (): DeviceDetailsRow[] => [
      {
        label: m.devices_details_label_last_ip(),
        value: isPresent(device.last_connected_ip) ? (
          formatIpForDisplay(device.last_connected_ip)
        ) : (
          <UnknownBadge />
        ),
      },
      {
        label: m.devices_details_label_last_date(),
        value: isPresent(device.last_connected_at) ? (
          displayDate(device.last_connected_at, lastConnectionDateFormat)
        ) : (
          <UnknownBadge />
        ),
      },
      {
        label: m.devices_details_label_connected_through(),
        value: device.last_connected_location ?? <UnknownBadge />,
      },
    ],
    [device],
  );

  const attestationRows = useMemo((): DeviceDetailsRow[] => {
    const { attestation_status: attestationStatus, verification } = device;
    return [
      {
        label: m.devices_col_attestation(),
        value: isPresent(attestationStatus)
          ? attestationStatusText[attestationStatus]()
          : null,
      },
      {
        label: m.devices_col_verification(),
        value: isPresent(verification) ? verificationText[verification]() : null,
      },
    ];
  }, [device]);

  return (
    <>
      <div className="body">
        <DeviceDetailsSection
          icon="devices"
          title={m.devices_details_section_general()}
          rows={generalRows}
        />
        <Divider spacing={ThemeSpacing.Xl} />
        <DeviceDetailsSection
          icon="device-ip"
          title={m.devices_details_section_last_connection()}
          rows={lastConnectionRows}
        />
        {isEnterprise && (
          <>
            <Divider spacing={ThemeSpacing.Xl} />
            <DeviceDetailsSection
              icon="connected-devices"
              title={m.devices_details_section_attestation()}
              rows={attestationRows}
            />
          </>
        )}
      </div>
      <div className="footer">
        <ButtonMenu
          variant="outlined"
          text={m.devices_details_actions()}
          iconRight="arrow-small"
          iconRightRotation="down"
          placement="top-start"
          testId="device-details-actions"
          menuItems={getMenuItems({
            devices: [device],
            testIdPrefix: 'device-details',
          })}
        />
        <Button variant="secondary" text={m.controls_close()} onClick={onClose} />
      </div>
    </>
  );
};
