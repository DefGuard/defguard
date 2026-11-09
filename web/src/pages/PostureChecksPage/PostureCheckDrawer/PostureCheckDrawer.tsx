import './PostureCheckDrawer.scss';
import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { Fragment, Suspense } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { ButtonMenu } from '../../../shared/defguard-ui/components/ButtonMenu/MenuButton';
import { Chip } from '../../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { DrawerModal } from '../../../shared/defguard-ui/components/DrawerModal/DrawerModal';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { IconKind } from '../../../shared/defguard-ui/components/Icon/icon-types';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import {
  getDevicePostureQueryOptions,
  getLocationsQueryOptions,
} from '../../../shared/query';
import {
  buildAttestationSections,
  buildOsSections,
  type OsSection,
} from '../../../shared/utils/postureInfo';
import { buildPostureCheckMenuItems } from '../postureCheckMenu';
import { buildFilteredLocationOptions, type PostureCheckRow } from '../postureChecks';

type ContentProps = {
  row: PostureCheckRow;
  onClose: () => void;
};

const PostureCheckDrawerContent = ({ row, onClose }: ContentProps) => {
  const navigate = useNavigate();
  const { data: postureCheck } = useSuspenseQuery(getDevicePostureQueryOptions(row.id));
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);

  const { mutateAsync: assignLocationsAsync } = useMutation({
    mutationFn: (locationIds: number[]) =>
      api.devicePosture.setLocationsForDevicePosture(row.id, locationIds),
    meta: {
      invalidate: [['device-posture'], ['network'], ['activity-log']],
    },
    onSuccess: () => {
      Snackbar.default(m.modal_assign_posture_check_locations_success());
    },
    onError: () => {
      Snackbar.error(m.modal_assign_posture_check_locations_error());
    },
  });

  const { mutate: duplicatePosture } = useMutation({
    mutationFn: api.devicePosture.duplicateDevicePosture,
    meta: {
      invalidate: [['device-posture'], ['network'], ['activity-log']],
    },
    onSuccess: (response) => {
      const posture = response.data;
      Snackbar.default(m.posture_checks_duplication_success());
      navigate({
        to: '/acl/posture-checks/$postureCheckId/edit',
        params: {
          postureCheckId: String(posture.id),
        },
      });
    },
    onError: () => {
      Snackbar.error(m.posture_checks_duplication_failed());
    },
  });
  const locationOptions = buildFilteredLocationOptions(locations);
  const assignedLocationNames = locationOptions
    .filter((loc) => row.locations.includes(loc.id))
    .map((loc) => loc.label);

  const osSections = buildOsSections(postureCheck);
  const attestationSections = buildAttestationSections(postureCheck);

  const menuItems = buildPostureCheckMenuItems({
    row,
    locationOptions,
    navigate,
    assignLocations: assignLocationsAsync,
    onAfterEdit: onClose,
    onAfterDelete: onClose,
    duplicatePosture: () => duplicatePosture(row.id),
  });

  return (
    <>
      <div className="posture-check-drawer-body">
        {postureCheck?.description && (
          <>
            <p className="drawer-description">{postureCheck.description}</p>
            <Divider spacing={ThemeSpacing.Xl} />
          </>
        )}

        {osSections.map((section, idx) => (
          <Fragment key={section.name}>
            {idx > 0 && <Divider spacing={ThemeSpacing.Lg} />}
            <DrawerSection section={section} />
          </Fragment>
        ))}

        {osSections.length > 0 && <Divider spacing={ThemeSpacing.Lg} />}

        <div className="drawer-locations">
          <div className="locations-label">
            <Icon icon={IconKind.LocationTracking} />
            <span className="section-name">Locations</span>
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

        {attestationSections.map((section) => (
          <Fragment key={section.name}>
            <Divider spacing={ThemeSpacing.Lg} />
            <DrawerSection section={section} />
          </Fragment>
        ))}
      </div>

      <div className="posture-check-drawer-footer">
        <ButtonMenu
          variant="outlined"
          text="Actions"
          iconRight={IconKind.ArrowSmall}
          iconRightRotation="down"
          menuItems={menuItems}
          placement="top-start"
        />
        <Button variant="secondary" text="Close" onClick={onClose} />
      </div>
    </>
  );
};

const DrawerSection = ({ section }: { section: OsSection }) => (
  <div className="drawer-section">
    <div className="section-header">
      <Icon icon={section.icon} />
      <span className="section-name">{section.name}</span>
    </div>
    <div className="section-rows">
      {section.rows.map((detail) => (
        <div key={detail.label} className="section-row">
          <span className="section-row-label">{detail.label}</span>
          {Array.isArray(detail.value) ? (
            <div className="section-row-value-list">
              {detail.value.map((item) => (
                <span key={item} className="section-row-value">
                  {item}
                </span>
              ))}
            </div>
          ) : (
            <span className="section-row-value">{detail.value}</span>
          )}
        </div>
      ))}
    </div>
  </div>
);

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
