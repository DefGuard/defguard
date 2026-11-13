import './style.scss';
import { useQuery } from '@tanstack/react-query';
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
import {
  attestationStatusText,
  ConnectionBadge,
  DeviceStateBadge,
  platformText,
  systemText,
  unknownBadge,
  verificationText,
} from '../../deviceDisplay';
import { useDeviceActions } from '../../hooks/useDeviceActions';
import {
  type DeviceDetailsRow,
  DeviceDetailsSection,
} from './components/DeviceDetailsSection/DeviceDetailsSection';

const lastConnectionDateFormat = 'YYYY-MM-DD HH:mm';

const missingValuePlaceholder = '—';

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

const DrawerContent = ({
  device,
  onClose,
}: {
  device: DeviceOverview;
  onClose: () => void;
}) => {
  const { data: usersDisplay } = useQuery(getUsersDisplayQueryOptions);
  const getActionMenuItems = useDeviceActions();

  const { os_type: osType } = device;

  const generalRows: DeviceDetailsRow[] = [
    { label: m.devices_details_label_name(), value: device.name },
    ...(isPresent(device.user_id)
      ? [{ label: m.devices_col_user(), value: usersDisplay?.[device.user_id] }]
      : []),
    {
      label: m.devices_col_platform(),
      value: isPresent(osType) ? platformText(osType) : unknownBadge,
    },
    {
      label: m.devices_col_system(),
      value: isPresent(osType) ? systemText(osType, device.os_version) : unknownBadge,
    },
    {
      label: m.devices_col_client_version(),
      value: device.client_version ?? unknownBadge,
    },
    { label: m.devices_col_state(), value: <DeviceStateBadge state={device.state} /> },
    {
      label: m.devices_col_connection(),
      value: <ConnectionBadge connected={device.connected} />,
    },
  ];

  const lastConnectionRows: DeviceDetailsRow[] = [
    {
      label: m.devices_details_label_last_ip(),
      value: isPresent(device.last_connected_ip)
        ? formatIpForDisplay(device.last_connected_ip)
        : missingValuePlaceholder,
    },
    {
      label: m.devices_details_label_last_date(),
      value: isPresent(device.last_connected_at)
        ? displayDate(device.last_connected_at, lastConnectionDateFormat)
        : missingValuePlaceholder,
    },
    {
      label: m.devices_details_label_connected_through(),
      value: device.last_connected_location ?? missingValuePlaceholder,
    },
  ];

  const attestationRows: DeviceDetailsRow[] = [
    {
      label: m.devices_col_attestation(),
      value: attestationStatusText[device.attestation_status](),
    },
    {
      label: m.devices_col_verification(),
      value: verificationText[device.verification](),
    },
  ];

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
        <Divider spacing={ThemeSpacing.Xl} />
        <DeviceDetailsSection
          icon="connected-devices"
          title={m.devices_details_section_attestation()}
          rows={attestationRows}
        />
      </div>
      <div className="footer">
        <ButtonMenu
          variant="outlined"
          text={m.devices_details_actions()}
          iconRight="arrow-small"
          iconRightRotation="down"
          placement="top-start"
          testId="device-details-actions"
          menuItems={getActionMenuItems({
            devices: [device],
            testIdPrefix: 'device-details',
          })}
        />
        <Button variant="secondary" text={m.controls_close()} onClick={onClose} />
      </div>
    </>
  );
};
