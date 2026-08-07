import type { useNavigate } from '@tanstack/react-router';
import { m } from '../../paraglide/messages';
import { useSelectionModal } from '../../shared/components/modals/SelectionModal/useSelectionModal';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { IconKind } from '../../shared/defguard-ui/components/Icon';
import type { MenuItemsGroup } from '../../shared/defguard-ui/components/Menu/types';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { confirmPostureLocationChange } from '../../shared/utils/postureWarning';
import { getDeletePostureCheckModalData, type PostureCheckRow } from './postureChecks';

type LocationOption = SelectionOption<number>;

type BuildPostureCheckMenuArgs = {
  row: PostureCheckRow;
  locationOptions: LocationOption[];
  navigate: ReturnType<typeof useNavigate>;
  assignLocations: (locationIds: number[]) => Promise<unknown>;
  duplicatePosture: () => void;
  onAfterEdit?: () => void;
  onAfterDelete?: () => void;
};

export const buildPostureCheckMenuItems = ({
  row,
  locationOptions,
  navigate,
  assignLocations,
  duplicatePosture,
  onAfterEdit,
  onAfterDelete,
}: BuildPostureCheckMenuArgs): MenuItemsGroup[] => [
  {
    items: [
      {
        text: m.controls_edit(),
        icon: 'edit',
        onClick: () => {
          void navigate({
            to: '/acl/posture-checks/$postureCheckId/edit',
            params: { postureCheckId: String(row.id) },
          });
          onAfterEdit?.();
        },
      },
      {
        text: 'Duplicate',
        icon: IconKind.Duplicate,
        onClick: duplicatePosture,
      },
      {
        text: m.posture_checks_row_menu_assign_locations(),
        icon: 'add-location',
        onClick: () => {
          useSelectionModal.setState({
            isOpen: true,
            title: m.modal_assign_posture_check_locations_title(),
            options: locationOptions,
            selected: new Set(row.locations),
            onSubmit: (selected) => {
              const next = selected as number[];
              confirmPostureLocationChange({
                current: row.locations,
                next,
                options: locationOptions,
                actionPromise: () => assignLocations(next),
              });
            },
          });
        },
      },
    ],
  },
  {
    items: [
      {
        text: m.controls_delete(),
        icon: 'delete',
        variant: 'danger',
        onClick: () => {
          const assignedLocationNames = locationOptions
            .filter((location) => row.locations.includes(location.id))
            .map((location) => location.label);

          openModal(ModalName.ConfirmAction, {
            ...getDeletePostureCheckModalData(row, assignedLocationNames),
            onSuccess: () => {
              Snackbar.default(m.modal_delete_posture_check_success());
              onAfterDelete?.();
            },
            onError: () => {
              Snackbar.error(m.modal_delete_posture_check_error());
            },
          });
        },
      },
    ],
  },
];
