import './style.scss';

import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { Link, useNavigate, useParams } from '@tanstack/react-router';
import { useEffect, useMemo, useState } from 'react';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type {
  ApiDevicePosture,
  EditDevicePostureRequest,
  NetworkLocation,
} from '../../shared/api/types';
import { Breadcrumbs } from '../../shared/components/Breadcrumbs/Breadcrumbs';
import {
  ContextualHelpKey,
  ContextualHelpSidebar,
} from '../../shared/components/ContextualHelp';
import { EditHeader } from '../../shared/components/EditHeader/EditHeader';
import { EditPageControls } from '../../shared/components/EditPageControls/EditPageControls';
import { EditPageFormSection } from '../../shared/components/EditPageFormSection/EditPageFormSection';
import { LayoutGrid } from '../../shared/components/LayoutGrid/LayoutGrid';
import { Page } from '../../shared/components/Page/Page';
import {
  PostureCheckDefguardSection,
  type PostureCheckEditorLocationOption,
  PostureCheckGeneralSection,
  PostureCheckLocationsSection,
  PostureCheckOperatingSystemsSection,
} from '../../shared/components/postureChecksEditor/PostureCheckEditorSections';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import {
  getDevicePostureQueryOptions,
  getDevicePostureVersionMetadataQueryOptions,
  getLocationsQueryOptions,
} from '../../shared/query';
import { openPostureAssignmentWarning } from '../../shared/utils/postureWarning';
import { buildAddPostureCheckRequest } from '../AddPostureCheckWizardPage/payload';
import { getDeletePostureCheckModalData } from '../PostureChecksPage/postureChecks';
import {
  getPostureCheckVersionValues,
  type PostureCheckVersionValues,
} from '../PostureChecksPage/types';
import {
  type EditPostureCheckFormValues,
  getInitialEditPostureCheckFormValues,
  normalizeEditPostureCheckFormValues,
} from './form';

const buildLocationOptions = (locations: NetworkLocation[]) =>
  [...locations]
    .sort((left, right) => left.name.localeCompare(right.name))
    .map((location) => ({ id: location.id, label: location.name }));

const EditPostureCheckForm = ({
  postureCheck,
  locations,
  versionValues,
}: {
  postureCheck: ApiDevicePosture;
  locations: NetworkLocation[];
  versionValues: PostureCheckVersionValues;
}) => {
  const navigate = useNavigate();
  const defaults = useMemo(
    () => getInitialEditPostureCheckFormValues(postureCheck),
    [postureCheck],
  );
  const [values, setValues] = useState<EditPostureCheckFormValues>(defaults);
  const locationOptions = useMemo<PostureCheckEditorLocationOption[]>(
    () => buildLocationOptions(locations),
    [locations],
  );
  const assignedLocationOptions = useMemo(
    () => locationOptions.filter((location) => defaults.locations.has(location.id)),
    [defaults.locations, locationOptions],
  );

  useEffect(() => {
    setValues(defaults);
  }, [defaults]);

  const saveMutation = useMutation({
    mutationFn: async (nextValues: EditPostureCheckFormValues) => {
      const requestData: EditDevicePostureRequest = buildAddPostureCheckRequest({
        allowPrereleaseClient: nextValues.allowPrereleaseClient,
        configuredOperatingSystems: nextValues.configuredOperatingSystems,
        description: nextValues.description,
        minimumDesktopClientVersion: nextValues.minimumDesktopClientVersion,
        minimumMobileClientVersion: nextValues.minimumMobileClientVersion,
        name: nextValues.name,
        operatingSystemState: nextValues.operatingSystemState,
      });

      await api.devicePosture.editDevicePosture(postureCheck.id, requestData);
      await api.devicePosture.setLocationsForDevicePosture(
        postureCheck.id,
        Array.from(nextValues.locations),
      );
    },
    meta: {
      invalidate: [['device-posture'], ['network'], ['activity-log']],
    },
    onSuccess: () => {
      Snackbar.default(m.posture_checks_edit_save_success());
    },
    onError: () => {
      Snackbar.error(m.posture_checks_edit_save_failed());
    },
  });

  const isDefault = useMemo(
    () =>
      JSON.stringify(normalizeEditPostureCheckFormValues(values)) ===
      JSON.stringify(normalizeEditPostureCheckFormValues(defaults)),
    [defaults, values],
  );

  const rulesChanged = useMemo(() => {
    const normalize = (v: EditPostureCheckFormValues) =>
      JSON.stringify({
        configuredOperatingSystems: [...v.configuredOperatingSystems].sort(),
        operatingSystemState: Object.fromEntries(
          Object.entries(v.operatingSystemState).map(([os, state]) => [
            os,
            {
              ...state,
              conditions: [...state.conditions].sort(),
            },
          ]),
        ),
        minimumDesktopClientVersion: v.minimumDesktopClientVersion,
        minimumMobileClientVersion: v.minimumMobileClientVersion,
        allowPrereleaseClient: v.allowPrereleaseClient,
      });
    return normalize(values) !== normalize(defaults);
  }, [values, defaults]);

  const locationsChanged = useMemo(() => {
    if (values.locations.size !== defaults.locations.size) return true;
    for (const id of values.locations) {
      if (!defaults.locations.has(id)) return true;
    }
    return false;
  }, [values.locations, defaults.locations]);

  const handleSubmit = () => {
    if (rulesChanged || locationsChanged) {
      const locationAddedIds: number[] = [];
      const locationRemovedIds: number[] = [];

      if (locationsChanged) {
        for (const id of values.locations) {
          if (!defaults.locations.has(id)) locationAddedIds.push(id);
        }
        for (const id of defaults.locations) {
          if (!values.locations.has(id)) locationRemovedIds.push(id);
        }
      }

      const nameById = new Map(locationOptions.map((loc) => [loc.id, loc.label]));
      const addedNames = locationAddedIds.map((id) => nameById.get(id) ?? String(id));
      const removedNames = locationRemovedIds.map((id) => nameById.get(id) ?? String(id));

      if (rulesChanged && locationsChanged) {
        openPostureAssignmentWarning({
          kind: 'postures',
          added: addedNames,
          removed: removedNames,
          extraBody: m.modal_posture_rules_warning_body(),
          actionPromise: () => saveMutation.mutateAsync(values),
          onError: () => {
            Snackbar.error(m.posture_checks_edit_save_failed());
          },
        });
        return;
      }

      if (locationsChanged) {
        openPostureAssignmentWarning({
          kind: 'postures',
          added: addedNames,
          removed: removedNames,
          actionPromise: () => saveMutation.mutateAsync(values),
          onError: () => {
            Snackbar.error(m.posture_checks_edit_save_failed());
          },
        });
        return;
      }

      openModal(ModalName.ConfirmAction, {
        title: m.modal_posture_assignment_warning_title(),
        contentMd: m.modal_posture_rules_warning_body(),
        actionPromise: () => saveMutation.mutateAsync(values),
        submitProps: {
          text: m.controls_save_changes_anyway(),
          variant: 'critical',
        },
        onError: () => {
          Snackbar.error(m.posture_checks_edit_save_failed());
        },
      });
      return;
    }

    void saveMutation.mutateAsync(values);
  };

  const updateValues = (
    updater: (current: EditPostureCheckFormValues) => EditPostureCheckFormValues,
  ) => {
    setValues((current) => updater(current));
  };

  const saveDisabled =
    saveMutation.isPending || values.name.trim().length === 0 || isDefault;

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        handleSubmit();
      }}
    >
      <EditPageFormSection label={m.posture_checks_edit_general()}>
        <PostureCheckGeneralSection values={values} updateValues={updateValues} />
      </EditPageFormSection>
      <EditPageFormSection label={m.posture_checks_edit_operating_systems()}>
        <PostureCheckOperatingSystemsSection
          asCards={false}
          values={values}
          versionValues={versionValues}
          updateValues={updateValues}
        />
      </EditPageFormSection>
      <EditPageFormSection label={m.posture_checks_edit_defguard()}>
        <PostureCheckDefguardSection
          values={values}
          versionValues={versionValues}
          updateValues={updateValues}
        />
      </EditPageFormSection>
      <EditPageFormSection label={m.posture_checks_edit_locations()}>
        <PostureCheckLocationsSection
          locationOptions={assignedLocationOptions}
          values={values}
          updateValues={updateValues}
        />
      </EditPageFormSection>
      <EditPageControls
        deleteProps={{
          text: m.controls_delete(),
          disabled: saveMutation.isPending,
          onClick: () => {
            const assignedLocationNames = locationOptions
              .filter((loc) => postureCheck.locations.includes(loc.id))
              .map((loc) => loc.label);

            openModal(ModalName.ConfirmAction, {
              ...getDeletePostureCheckModalData(
                { id: postureCheck.id, name: postureCheck.name },
                assignedLocationNames,
              ),
              onSuccess: () => {
                Snackbar.default(m.posture_checks_edit_delete_success());
                navigate({ to: '/acl/posture-checks', replace: true });
              },
              onError: () => {
                Snackbar.error(m.posture_checks_edit_delete_failed());
              },
            });
          },
        }}
        cancelProps={{
          disabled: saveMutation.isPending,
          onClick: () => {
            navigate({ to: '/acl/posture-checks' });
          },
        }}
        submitProps={{
          disabled: saveDisabled,
          loading: saveMutation.isPending,
          onClick: () => {
            handleSubmit();
          },
        }}
      />
    </form>
  );
};

export const EditPostureCheckPage = () => {
  const { postureCheckId } = useParams({
    from: '/_authorized/_default/acl/posture-checks/$postureCheckId/edit',
  });
  const { data: postureCheck } = useSuspenseQuery(
    getDevicePostureQueryOptions(Number(postureCheckId)),
  );
  const { data: versionMetadata } = useSuspenseQuery(
    getDevicePostureVersionMetadataQueryOptions,
  );
  const { data: locations } = useSuspenseQuery(getLocationsQueryOptions);
  const versionValues = useMemo(
    () => getPostureCheckVersionValues(versionMetadata),
    [versionMetadata],
  );
  const breadcrumbsLinks = useMemo(
    () => [
      <Link key="list" to="/acl/posture-checks">
        {m.cmp_nav_item_posture_checks()}
      </Link>,
      <Link
        key="edit"
        to="/acl/posture-checks/$postureCheckId/edit"
        params={{ postureCheckId }}
      >
        {postureCheck.name}
      </Link>,
    ],
    [postureCheck.name, postureCheckId],
  );

  return (
    <Page className="edit-posture-check-page" title={m.cmp_nav_item_posture_checks()}>
      <Breadcrumbs links={breadcrumbsLinks} />
      <LayoutGrid>
        <div className="main-content">
          <EditHeader title={m.posture_checks_edit_title({ name: postureCheck.name })} />
          <div className="card">
            <EditPostureCheckForm
              postureCheck={postureCheck}
              locations={locations}
              versionValues={versionValues}
            />
          </div>
        </div>
        <div className="helpers">
          <ContextualHelpSidebar pageKey={ContextualHelpKey.PostureChecksEdit} />
        </div>
      </LayoutGrid>
    </Page>
  );
};
