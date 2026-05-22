import './PostureCheckDrawer.scss';
import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { Fragment, Suspense } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { ApiDevicePosture } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { ButtonMenu } from '../../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { Chip } from '../../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { DrawerModal } from '../../../shared/defguard-ui/components/DrawerModal/DrawerModal';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { IconKind } from '../../../shared/defguard-ui/components/Icon/icon-types';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  getDevicePostureQueryOptions,
  getLocationsQueryOptions,
} from '../../../shared/query';
import { buildPostureCheckMenuItems } from '../postureCheckMenu';
import type { PostureCheckRow } from '../postureChecks';

type OsDetailRow = {
  label: string;
  value: string | string[];
};

type OsSection = {
  icon: (typeof IconKind)[keyof typeof IconKind];
  name: string;
  rows: OsDetailRow[];
};

const getWindowsSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'windows' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.ad_domain_joined_required) otherItems.push('Connected to Active Directory');
  if (rule.antivirus_required) otherItems.push('Antivirus installed');
  if (rule.disk_encryption_required) otherItems.push('Disk encryption enabled');

  const rows: OsDetailRow[] = [];
  if (rule.min_os_version !== null) {
    rows.push({ label: 'Version', value: `Windows ${rule.min_os_version} and higher` });
  }
  if (rule.windows_security_update_max_age !== null) {
    rows.push({
      label: 'Update',
      value: `Minimum ${rule.windows_security_update_max_age} month update`,
    });
  }
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Windows, name: 'Windows', rows };
};

const getMacosSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'macos' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.disk_encryption_required) otherItems.push('Disk encryption enabled');
  if (rule.device_integrity_required) otherItems.push('Device integrity');

  const rows: OsDetailRow[] = [];
  if (rule.min_os_version !== null) {
    rows.push({ label: 'Version', value: `macOS ${rule.min_os_version} and higher` });
  }
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Apple, name: 'macOS', rows };
};

const getLinuxSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'linux' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.disk_encryption_required) otherItems.push('Disk encryption enabled');

  const rows: OsDetailRow[] = [];
  if (rule.min_kernel_version !== null) {
    rows.push({
      label: 'Version',
      value: `Kernel ${rule.min_kernel_version} and higher`,
    });
  }
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Linux, name: 'Linux', rows };
};

const getIosSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'ios' }>,
): OsSection => {
  const rows: OsDetailRow[] = [];
  if (rule.min_os_version !== null) {
    rows.push({ label: 'Version', value: `iOS ${rule.min_os_version}+` });
  }

  return { icon: IconKind.AppStore, name: 'iOS', rows };
};

const getAndroidSection = (
  rule: Extract<ApiDevicePosture['os_rules'][number], { os_type: 'android' }>,
): OsSection => {
  const otherItems: string[] = [];
  if (rule.device_integrity_required) otherItems.push('Device integrity');

  const rows: OsDetailRow[] = [];
  if (rule.min_os_version !== null) {
    rows.push({ label: 'Version', value: `Android ${rule.min_os_version}+` });
  }
  if (otherItems.length > 0) {
    rows.push({ label: 'Other', value: otherItems });
  }

  return { icon: IconKind.Android, name: 'Android', rows };
};

const getDefguardSection = (posture: ApiDevicePosture): OsSection | null => {
  if (!posture.min_client_version && !posture.allow_prerelease_client) return null;

  const rows: OsDetailRow[] = [];
  if (posture.min_client_version) {
    rows.push({
      label: 'Version',
      value: `Defguard ${posture.min_client_version} and higher`,
    });
  }
  if (posture.allow_prerelease_client) {
    rows.push({ label: 'Other', value: 'Pre-release allowed' });
  }

  return { icon: IconKind.Defguard, name: 'Defguard', rows };
};

const buildOsSections = (posture: ApiDevicePosture): OsSection[] => {
  const sections: OsSection[] = [];

  for (const rule of posture.os_rules ?? []) {
    switch (rule.os_type) {
      case 'windows':
        sections.push(getWindowsSection(rule));
        break;
      case 'macos':
        sections.push(getMacosSection(rule));
        break;
      case 'linux':
        sections.push(getLinuxSection(rule));
        break;
      case 'ios':
        sections.push(getIosSection(rule));
        break;
      case 'android':
        sections.push(getAndroidSection(rule));
        break;
    }
  }

  const defguardSection = getDefguardSection(posture);
  if (defguardSection) sections.push(defguardSection);

  return sections;
};

type ContentProps = {
  row: PostureCheckRow;
  onClose: () => void;
};

const PostureCheckDrawerContent = ({ row, onClose }: ContentProps) => {
  const navigate = useNavigate();
  const { data: postureCheck } = useSuspenseQuery(getDevicePostureQueryOptions(row.id));
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);

  const { mutate: assignLocations } = useMutation({
    mutationFn: (locationIds: number[]) =>
      api.devicePosture.setLocationsForDevicePosture(row.id, locationIds),
    meta: {
      invalidate: [['device-posture'], ['network']],
    },
    onSuccess: () => {
      Snackbar.default(m.modal_assign_posture_check_locations_success());
    },
    onError: () => {
      Snackbar.error(m.modal_assign_posture_check_locations_error());
    },
  });

  const locationOptions = locations.map((loc) => ({
    id: loc.id,
    label: loc.name,
    searchFields: [loc.name, ...loc.address],
  }));

  const assignedLocationNames = locationOptions
    .filter((loc) => row.locations.includes(loc.id))
    .map((loc) => loc.label);

  const osSections = buildOsSections(postureCheck);

  const menuItems = buildPostureCheckMenuItems({
    row,
    locationOptions,
    navigate,
    assignLocations,
    onAfterEdit: onClose,
    onAfterDelete: onClose,
  });

  return (
    <>
      <div className="posture-check-drawer-body">
        {postureCheck?.description && (
          <>
            <p className="drawer-block drawer-description">{postureCheck.description}</p>
            <Divider />
          </>
        )}

        {osSections.map((section, idx) => (
          <Fragment key={section.name}>
            {idx > 0 && <Divider />}
            <div className="drawer-block drawer-os-section">
              <div className="os-header">
                <Icon icon={section.icon} />
                <span className="os-name">{section.name}</span>
              </div>
              <div className="os-rows">
                {section.rows.map((detail) => (
                  <div key={detail.label} className="os-row">
                    <span className="os-row-label">{detail.label}</span>
                    {Array.isArray(detail.value) ? (
                      <div className="os-row-value-list">
                        {detail.value.map((item) => (
                          <span key={item} className="os-row-value">
                            {item}
                          </span>
                        ))}
                      </div>
                    ) : (
                      <span className="os-row-value">{detail.value}</span>
                    )}
                  </div>
                ))}
              </div>
            </div>
          </Fragment>
        ))}

        {(osSections.length > 0 || postureCheck?.description) && <Divider />}

        <div className="drawer-block drawer-locations">
          <div className="locations-label">
            <Icon icon={IconKind.LocationTracking} />
            <span className="os-name">Locations</span>
          </div>
          <div className="locations-value">
            {assignedLocationNames.length > 0 ? (
              <div className="locations-chips">
                {assignedLocationNames.map((name) => (
                  <Chip key={name} text={name} />
                ))}
              </div>
            ) : (
              <span className="locations-empty">Not used in any location</span>
            )}
          </div>
        </div>
      </div>

      <div className="posture-check-drawer-footer">
        <ButtonMenu
          variant="outlined"
          text="Actions"
          iconRight={IconKind.ArrowSmall}
          iconRightRotation="down"
          menuItems={menuItems}
        />
        <Button variant="secondary" text="Close" onClick={onClose} />
      </div>
    </>
  );
};

type Props = {
  selectedRow: PostureCheckRow | null;
  onClose: () => void;
};

export const PostureCheckDrawer = ({ selectedRow, onClose }: Props) => {
  return (
    <DrawerModal
      isOpen={selectedRow !== null}
      onClose={onClose}
      title={selectedRow?.name ?? ''}
      contentClassName="posture-check-drawer"
    >
      {selectedRow && (
        <Suspense>
          <PostureCheckDrawerContent row={selectedRow} onClose={onClose} />
        </Suspense>
      )}
    </DrawerModal>
  );
};
