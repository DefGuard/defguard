import './style.scss';

import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from '@tanstack/react-router';
import { cloneDeep, omit } from 'lodash-es';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type EditNetworkLocation,
  LocationMfaMode,
  LocationServiceMode,
  type NetworkLocation,
} from '../../shared/api/types';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { EditPageControls } from '../../shared/components/EditPageControls/EditPageControls';
import { EditPageFormSection } from '../../shared/components/EditPageFormSection/EditPageFormSection';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { externalLink } from '../../shared/constants';

import { InfoBanner } from '../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
import { getLicenseInfoQueryOptions, getLocationQueryOptions } from '../../shared/query';
import {
  canUseBusinessFeature,
  canUseEnterpriseFeature,
} from '../../shared/utils/license';
import { Validate } from '../../shared/validate';

export const EditLocationPage = () => {
  const { locationId: paramsId } = useParams({
    from: '/_authorized/_default/locations/$locationId/edit',
  });
  const { data: location } = useSuspenseQuery(getLocationQueryOptions(Number(paramsId)));

  return (
    <EditPage
      id="edit-location-page"
      pageTitle="Locations"
      headerProps={{
        title: `Edit ${location.name} location`,
      }}
    >
      <EditLocationForm location={location} />
    </EditPage>
  );
};

const LocationFirewall = {
  Disabled: 'disabled',
  Allow: 'allow',
  Deny: 'deny',
} as const;

type LocationFirewallValue = 'disabled' | 'allow' | 'deny';

const locationToFirewall = (location: NetworkLocation): LocationFirewallValue => {
  if (!location.acl_enabled) return 'disabled';
  if (location.acl_default_allow) return 'allow';
  return 'deny';
};

const peerDisconnectThresholdMinimum = 120;

const formSchema = z
  .object({
    name: z.string(m.form_error_required()).min(1, m.form_error_required()),
    address: z
      .string(m.form_error_required())
      .trim()
      .min(1, m.form_error_required())
      .refine(
        (val) => Validate.any(val, [Validate.CIDRv4, Validate.CIDRv6], true),
        m.form_error_invalid(),
      ),
    endpoint: z
      .string(m.form_error_required())
      .trim()
      .min(1, m.form_error_required())
      .refine((val) =>
        Validate.any(val, [
          Validate.IPv4,
          Validate.IPv6,
          Validate.Domain,
          Validate.Hostname,
        ]),
      ),
    port: z.number(m.form_error_required()).max(65535, m.form_error_port_max()),
    allowed_ips: z
      .string()
      .trim()
      .nullable()
      .refine((val) => {
        if (!val) return true;
        return Validate.any(
          val,
          [
            Validate.IPv4,
            Validate.IPv6,
            (v) => Validate.CIDRv4(v, true),
            (v) => Validate.CIDRv6(v, true),
          ],
          true,
        );
      }, m.form_error_invalid()),
    dns: z
      .string()
      .trim()
      .nullable()
      .refine((val) => {
        if (!val) return true;
        return Validate.any(
          val,
          [Validate.IPv4, Validate.IPv6, Validate.Domain, Validate.Hostname],
          true,
        );
      }),
    peer_disconnect_threshold: z.number().nullable(),
    keepalive_interval: z
      .number(m.form_error_required())
      .max(65535, m.form_error_port_max()),
    mtu: z.number(m.form_error_required()).min(72).max(0xffffffff),
    fwmark: z.number(m.form_error_required()).min(0).max(0xffffffff),
    allow_all_groups: z.boolean(),
    allowed_groups: z.array(
      z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
    ),
    location_mfa_mode: z.enum(LocationMfaMode),
    service_location_mode: z.enum(LocationServiceMode),
    firewall: z.enum(LocationFirewall),
  })
  .superRefine((value, context) => {
    if (value.location_mfa_mode === LocationMfaMode.Disabled) {
      return;
    }

    if (value.peer_disconnect_threshold === null) {
      context.addIssue({
        code: 'custom',
        path: ['peer_disconnect_threshold'],
        message: m.form_error_required(),
      });
      return;
    }

    if (value.peer_disconnect_threshold < peerDisconnectThresholdMinimum) {
      context.addIssue({
        code: 'custom',
        path: ['peer_disconnect_threshold'],
        message: m.form_min_value({ value: peerDisconnectThresholdMinimum }),
      });
    }
  });

type FormFields = z.infer<typeof formSchema>;

type DisconnectRelevantField =
  | 'address'
  | 'port'
  | 'mtu'
  | 'fwmark'
  | 'location_mfa_mode'
  | 'service_location_mode'
  | 'allow_all_groups'
  | 'allowed_groups';

type DisconnectRelevantLocationData = Pick<
  EditNetworkLocation,
  | 'address'
  | 'port'
  | 'mtu'
  | 'fwmark'
  | 'location_mfa_mode'
  | 'service_location_mode'
  | 'allow_all_groups'
  | 'allowed_groups'
>;

const normalizeCommaSeparatedValues = (value: string) =>
  value
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean)
    .sort();

const normalizeSelectedGroups = (groups: string[]) =>
  [...new Set(groups.map((group) => group.trim()).filter(Boolean))].sort();

const areEqualStringArrays = (left: string[], right: string[]) =>
  left.length === right.length && left.every((value, index) => value === right[index]);

const buildLocationSubmissionData = (
  value: FormFields,
  location: NetworkLocation,
): EditNetworkLocation => {
  const normalizedValue = cloneDeep(value);

  if (normalizedValue.location_mfa_mode !== LocationMfaMode.Disabled) {
    normalizedValue.service_location_mode = LocationServiceMode.Disabled;
  }

  return {
    ...omit(normalizedValue, ['firewall']),
    allowed_ips: normalizedValue.allowed_ips ?? '',
    acl_default_allow: normalizedValue.firewall === LocationFirewall.Allow,
    acl_enabled: normalizedValue.firewall !== LocationFirewall.Disabled,
    peer_disconnect_threshold:
      normalizedValue.peer_disconnect_threshold ?? location.peer_disconnect_threshold,
  };
};

const getDisconnectRelevantLocationData = (
  value: EditNetworkLocation,
): DisconnectRelevantLocationData => ({
  address: value.address,
  port: value.port,
  mtu: value.mtu,
  fwmark: value.fwmark,
  location_mfa_mode: value.location_mfa_mode,
  service_location_mode: value.service_location_mode,
  allow_all_groups: value.allow_all_groups,
  allowed_groups: value.allow_all_groups ? [] : value.allowed_groups,
});

const getDisconnectRelevantFieldLabel = (field: DisconnectRelevantField): string => {
  switch (field) {
    case 'address':
      return m.location_edit_field_address();
    case 'port':
      return m.location_edit_field_port();
    case 'mtu':
      return m.location_edit_field_mtu();
    case 'fwmark':
      return m.location_edit_field_fwmark();
    case 'location_mfa_mode':
      return m.location_edit_field_location_mfa_mode();
    case 'service_location_mode':
      return m.location_edit_field_service_location_mode();
    case 'allow_all_groups':
      return m.location_edit_field_allow_all_groups();
    case 'allowed_groups':
      return m.location_edit_field_allowed_groups();
  }
};

const getDisconnectRelevantChangedFields = (
  original: DisconnectRelevantLocationData,
  submitted: DisconnectRelevantLocationData,
): DisconnectRelevantField[] => {
  const changedFields: DisconnectRelevantField[] = [];

  if (
    !areEqualStringArrays(
      normalizeCommaSeparatedValues(original.address),
      normalizeCommaSeparatedValues(submitted.address),
    )
  ) {
    changedFields.push('address');
  }

  if (original.port !== submitted.port) {
    changedFields.push('port');
  }

  if (original.mtu !== submitted.mtu) {
    changedFields.push('mtu');
  }

  if (original.fwmark !== submitted.fwmark) {
    changedFields.push('fwmark');
  }

  if (original.location_mfa_mode !== submitted.location_mfa_mode) {
    changedFields.push('location_mfa_mode');
  }

  if (original.service_location_mode !== submitted.service_location_mode) {
    changedFields.push('service_location_mode');
  }

  if (original.allow_all_groups !== submitted.allow_all_groups) {
    changedFields.push('allow_all_groups');
  }

  if (
    !submitted.allow_all_groups &&
    !areEqualStringArrays(
      normalizeSelectedGroups(original.allowed_groups),
      normalizeSelectedGroups(submitted.allowed_groups),
    )
  ) {
    changedFields.push('allowed_groups');
  }

  return changedFields;
};

const EditLocationForm = ({ location }: { location: NetworkLocation }) => {
  const navigate = useNavigate();

  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const canUseEnterprise = useMemo(() => {
    if (licenseInfo === undefined) return undefined;
    return canUseEnterpriseFeature(licenseInfo).result;
  }, [licenseInfo]);
  const canUseBusiness = useMemo(() => {
    if (licenseInfo === undefined) return undefined;
    return canUseBusinessFeature(licenseInfo).result;
  }, [licenseInfo]);
  const serviceLocationLocked = isPresent(canUseEnterprise) && !canUseEnterprise;
  const firewallLocked = isPresent(canUseBusiness) && !canUseBusiness;

  const serviceLocationLabelContent = useMemo(() => {
    if (!serviceLocationLocked) return undefined;
    return (
      <>
        <p>{m.license_enterprise_required()}</p>
        <a href={externalLink.defguard.pricing} target="_blank" rel="noreferrer">
          {m.license_upgrade_to_unlock()}
        </a>
      </>
    );
  }, [serviceLocationLocked]);

  const firewallLabelContent = useMemo(() => {
    if (!firewallLocked) return undefined;
    return (
      <>
        <p>{m.license_business_required()}</p>
        <a href={externalLink.defguard.pricing} target="_blank" rel="noreferrer">
          {m.license_upgrade_to_unlock()}
        </a>
      </>
    );
  }, [firewallLocked]);

  const { data: groupsOptions } = useQuery({
    queryFn: api.group.getGroups,
    queryKey: ['group'],
    select: (groups) =>
      groups.map(
        (group): SelectionOption<string> => ({
          id: group,
          label: group,
        }),
      ),
  });

  const { mutateAsync: editLocation } = useMutation({
    mutationFn: api.location.editLocation,
    meta: {
      invalidate: ['network'],
    },
    onSuccess: () => {
      navigate({
        to: '/locations',
        replace: true,
      });
    },
    onError: () => {
      Snackbar.error(m.location_edit_failed());
    },
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      name: location.name,
      address: location.address.join(','),
      allow_all_groups: location.allow_all_groups,
      allowed_groups: [...location.allowed_groups],
      allowed_ips: location.allowed_ips.join(','),
      dns: location.dns,
      endpoint: location.endpoint,
      keepalive_interval: location.keepalive_interval,
      mtu: location.mtu,
      fwmark: location.fwmark,
      location_mfa_mode: location.location_mfa_mode,
      peer_disconnect_threshold: location.peer_disconnect_threshold,
      port: location.port,
      service_location_mode: location.service_location_mode,
      firewall: locationToFirewall(location),
    }),
    [location],
  );

  const submitLocationChanges = async (value: FormFields) => {
    await editLocation({
      id: location.id,
      data: buildLocationSubmissionData(value, location),
    });
  };

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const changedFields = getDisconnectRelevantChangedFields(
        getDisconnectRelevantLocationData(
          buildLocationSubmissionData(defaultValues, location),
        ),
        getDisconnectRelevantLocationData(buildLocationSubmissionData(value, location)),
      );

      if (changedFields.length > 0) {
        openModal(ModalName.ConfirmAction, {
          title: m.modal_edit_location_disconnect_warning_title(),
          contentMd: m.modal_edit_location_disconnect_warning_body({
            fields: changedFields
              .map((field) => `- ${getDisconnectRelevantFieldLabel(field)}`)
              .join('\n'),
          }),
          actionPromise: () => submitLocationChanges(value),
          submitProps: {
            text: m.controls_save_changes(),
          },
        });
        return;
      }

      await submitLocationChanges(value);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <EditPageFormSection label="Public facing data">
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Location name" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="port">
            {(field) => (
              <field.FormInput
                required
                label={m.location_edit_field_port()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="endpoint">
            {(field) => (
              <field.FormInput required label="Gateway IP address or domain name" />
            )}
          </form.AppField>
        </EditPageFormSection>
        <EditPageFormSection label="Internal VPN settings">
          {location.has_devices && (
            <>
              <InfoBanner
                icon="info-outlined"
                variant="warning"
                text={m.location_edit_addresses_rewrite_warning()}
              />
              <SizedBox height={ThemeSpacing.Lg} />
            </>
          )}
          <form.AppField name="address">
            {(field) => (
              <field.FormInput required label={m.location_edit_field_address()} />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="allowed_ips">
            {(field) => <field.FormInput label="Allowed IPs" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="dns">
            {(field) => <field.FormInput label="DNS" />}
          </form.AppField>
        </EditPageFormSection>
        <EditPageFormSection label="Network settings">
          <form.AppField name="keepalive_interval">
            {(field) => (
              <field.FormInput
                required
                label="Keep alive interval (seconds)"
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="mtu">
            {(field) => (
              <field.FormInput label={m.location_edit_field_mtu()} type="number" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="fwmark">
            {(field) => (
              <field.FormInput label={m.location_edit_field_fwmark()} type="number" />
            )}
          </form.AppField>
        </EditPageFormSection>
        <form.Subscribe
          selector={(s) =>
            s.values.service_location_mode !== LocationServiceMode.Disabled
          }
        >
          {(isServiceLocation) => (
            <>
              {isServiceLocation && (
                <InfoBanner
                  icon="info-outlined"
                  variant="warning"
                  text={m.location_edit_service_location_mfa_warning()}
                />
              )}
              <EditPageFormSection label={m.location_edit_field_location_mfa_mode()}>
                <form.AppField
                  name="location_mfa_mode"
                  validators={{
                    onChangeListenTo: ['service_location_mode'],
                  }}
                  listeners={{
                    onChange: ({ value, fieldApi }) => {
                      const service = fieldApi.form.getFieldValue(
                        'service_location_mode',
                      );
                      if (
                        value !== LocationMfaMode.Disabled &&
                        service !== LocationServiceMode.Disabled
                      ) {
                        fieldApi.form.setFieldValue(
                          'service_location_mode',
                          LocationServiceMode.Disabled,
                        );
                      }
                    },
                  }}
                >
                  {(field) => (
                    <>
                      <field.FormRadio
                        value={LocationMfaMode.Disabled}
                        text="Do not enforce MFA"
                        disabled={isServiceLocation}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationMfaMode.Internal}
                        text="Internal MFA"
                        disabled={isServiceLocation}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationMfaMode.External}
                        text="External MFA"
                        disabled={isServiceLocation}
                      />
                    </>
                  )}
                </form.AppField>
                <form.Subscribe
                  selector={(state) =>
                    state.values.location_mfa_mode !== LocationMfaMode.Disabled
                  }
                >
                  {(showDisconnectThreshold) =>
                    showDisconnectThreshold ? (
                      <>
                        <SizedBox height={ThemeSpacing.Xl2} />
                        <form.AppField name="peer_disconnect_threshold">
                          {(field) => (
                            <field.FormInput
                              required
                              label="Client disconnect threshold (seconds)"
                              type="number"
                            />
                          )}
                        </form.AppField>
                      </>
                    ) : null
                  }
                </form.Subscribe>
              </EditPageFormSection>
            </>
          )}
        </form.Subscribe>
        <form.Subscribe
          selector={(s) => s.values.location_mfa_mode !== LocationMfaMode.Disabled}
        >
          {(mfaEnabled) => (
            <form.AppField
              name="service_location_mode"
              validators={{ onChangeListenTo: ['location_mfa_mode'] }}
              listeners={{
                onChange: ({ value, fieldApi }) => {
                  const mfa = fieldApi.form.getFieldValue('location_mfa_mode');
                  if (
                    value !== LocationServiceMode.Disabled &&
                    mfa !== LocationMfaMode.Disabled
                  ) {
                    fieldApi.form.setFieldValue(
                      'location_mfa_mode',
                      LocationMfaMode.Disabled,
                    );
                  }
                },
              }}
            >
              {(field) => {
                return (
                  <>
                    {mfaEnabled && (
                      <InfoBanner
                        variant="warning"
                        icon="info-outlined"
                        text={m.location_edit_mfa_service_location_warning()}
                      />
                    )}
                    <EditPageFormSection
                      label={m.location_edit_field_service_location_mode()}
                      labelContent={serviceLocationLabelContent}
                    >
                      <field.FormRadio
                        value={LocationServiceMode.Disabled}
                        text="Regular location"
                        disabled={mfaEnabled || serviceLocationLocked}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationServiceMode.Prelogon}
                        text="Service location: Pre-logon"
                        disabled={mfaEnabled || serviceLocationLocked}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationServiceMode.Alwayson}
                        text="Service location: Always on"
                        disabled={mfaEnabled || serviceLocationLocked}
                      />
                    </EditPageFormSection>
                  </>
                );
              }}
            </form.AppField>
          )}
        </form.Subscribe>
        <form.Subscribe selector={(state) => state.values.allow_all_groups}>
          {(allowAllGroups) => (
            <EditPageFormSection label="Location Access">
              {isPresent(groupsOptions) && (
                <form.AppField name="allowed_groups">
                  {(field) => (
                    <field.FormSelectMultiple
                      options={groupsOptions}
                      counterText={(count) => `+${count} groups`}
                      editText="Edit groups"
                      modalTitle={m.location_edit_field_allowed_groups()}
                      toggleText={m.location_edit_field_allow_all_groups()}
                      toggleValue={allowAllGroups}
                      onToggleChange={(value) => {
                        form.setFieldValue('allow_all_groups', value);
                      }}
                    />
                  )}
                </form.AppField>
              )}
            </EditPageFormSection>
          )}
        </form.Subscribe>
        <EditPageFormSection label="Firewall" labelContent={firewallLabelContent}>
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Disabled}
                text="Disable firewall option"
                disabled={firewallLocked}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Allow}
                text="Users/devices can access all resources unless limited by ACL rules."
                disabled={firewallLocked}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Deny}
                text="All traffic not explicitly allowed by an ACL rule will be blocked."
                disabled={firewallLocked}
              />
            )}
          </form.AppField>
        </EditPageFormSection>
        <form.Subscribe
          selector={(form) => ({
            isSubmitting: form.isSubmitting,
            isDefault: form.isPristine || form.isDefaultValue,
          })}
        >
          {({ isDefault, isSubmitting }) => (
            <EditPageControls
              deleteProps={{
                text: 'Delete location',
                onClick: () => {
                  openModal(ModalName.ConfirmAction, {
                    title: m.modal_delete_location_title(),
                    contentMd: m.modal_delete_location_body({ name: location.name }),
                    actionPromise: () => api.location.deleteLocation(location.id),
                    invalidateKeys: [['network'], ['enterprise_info']],
                    submitProps: { text: m.controls_delete(), variant: 'critical' },
                    onSuccess: () => {
                      Snackbar.default(m.location_delete_success());
                      navigate({ to: '/locations', replace: true });
                    },
                    onError: () => Snackbar.error(m.location_delete_failed()),
                  });
                },
                disabled: isSubmitting,
              }}
              cancelProps={{
                onClick: () => {
                  window.history.back();
                },
              }}
              submitProps={{
                loading: isSubmitting,
                disabled: isDefault,
                onClick: () => {
                  form.handleSubmit();
                },
              }}
            />
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
