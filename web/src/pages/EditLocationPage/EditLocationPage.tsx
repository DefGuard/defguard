import './style.scss';

import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from '@tanstack/react-router';
import { cloneDeep, omit } from 'lodash-es';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
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
      pageTitle={m.cmp_nav_item_locations()}
      headerProps={{
        title: m.location_edit_title({ name: location.name }),
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

const getSelectedGroupsCounterText = (count: number) => {
  if (count === 1) {
    return m.location_access_selected_group_count_one({ count });
  }

  return m.location_access_selected_group_count_other({ count });
};

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
      allowed_groups: location.allowed_groups,
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

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const clone = cloneDeep(value);
      if (clone.location_mfa_mode !== LocationMfaMode.Disabled) {
        clone.service_location_mode = LocationServiceMode.Disabled;
      }

      const peerDisconnectThreshold =
        clone.peer_disconnect_threshold ?? location.peer_disconnect_threshold;

      await editLocation({
        id: location.id,
        data: {
          ...omit(clone, ['firewall']),
          allow_all_groups: clone.allow_all_groups,
          allowed_groups: clone.allowed_groups,
          allowed_ips: clone.allowed_ips ?? '',
          acl_default_allow: clone.firewall === LocationFirewall.Allow,
          acl_enabled: !(clone.firewall === LocationFirewall.Disabled),
          peer_disconnect_threshold: peerDisconnectThreshold,
        },
      });
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
        <EditPageFormSection label={m.add_location_step_public_facing_data_label()}>
          <form.AppField name="name">
            {(field) => <field.FormInput required label={m.location_form_label_name()} />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="port">
            {(field) => (
              <field.FormInput
                required
                label={m.add_location_start_label_port()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="endpoint">
            {(field) => (
              <field.FormInput required label={m.add_location_start_label_endpoint()} />
            )}
          </form.AppField>
        </EditPageFormSection>
        <EditPageFormSection label={m.add_location_step_internal_vpn_label()}>
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
              <field.FormInput
                required
                label={m.add_location_internal_vpn_label_address()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="allowed_ips">
            {(field) => (
              <field.FormInput label={m.add_location_internal_vpn_label_allowed_ips()} />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="dns">
            {(field) => (
              <field.FormInput label={m.add_location_internal_vpn_label_dns()} />
            )}
          </form.AppField>
        </EditPageFormSection>
        <EditPageFormSection label={m.add_location_step_network_settings_label()}>
          <form.AppField name="keepalive_interval">
            {(field) => (
              <field.FormInput
                required
                label={m.location_network_label_keepalive_interval()}
                type="number"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="mtu">
            {(field) => (
              <field.FormInput label={m.location_network_label_mtu()} type="number" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="fwmark">
            {(field) => (
              <field.FormInput label={m.location_network_label_fwmark()} type="number" />
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
                  text={m.location_mfa_service_location_warning()}
                />
              )}
              <EditPageFormSection label={m.add_location_step_mfa_label()}>
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
                        text={m.add_location_mfa_disabled_title()}
                        disabled={isServiceLocation}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationMfaMode.Internal}
                        text={m.location_mfa_option_internal()}
                        disabled={isServiceLocation}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationMfaMode.External}
                        text={m.location_mfa_option_external()}
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
                              label={m.location_mfa_label_client_disconnect_threshold()}
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
                        text={m.location_service_mode_mfa_warning()}
                      />
                    )}
                    <EditPageFormSection
                      label={m.location_edit_section_location_type()}
                      labelContent={serviceLocationLabelContent}
                    >
                      <field.FormRadio
                        value={LocationServiceMode.Disabled}
                        text={m.location_service_mode_regular()}
                        disabled={mfaEnabled || serviceLocationLocked}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationServiceMode.Prelogon}
                        text={m.location_service_mode_prelogon()}
                        disabled={mfaEnabled || serviceLocationLocked}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationServiceMode.Alwayson}
                        text={m.location_service_mode_always_on()}
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
            <EditPageFormSection label={m.location_access_section_label()}>
              {isPresent(groupsOptions) && (
                <form.AppField name="allowed_groups">
                  {(field) => (
                    <field.FormSelectMultiple
                      options={groupsOptions}
                      counterText={getSelectedGroupsCounterText}
                      editText={m.location_access_edit_groups()}
                      modalTitle={m.location_access_select_allowed_groups()}
                      toggleText={m.location_access_all_groups_have_access()}
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
        <EditPageFormSection
          label={m.add_location_step_firewall_label()}
          labelContent={firewallLabelContent}
        >
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Disabled}
                text={m.location_firewall_option_disabled()}
                disabled={firewallLocked}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Allow}
                text={m.location_firewall_option_default_allow()}
                disabled={firewallLocked}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Deny}
                text={m.location_firewall_option_default_deny()}
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
                text: m.modal_delete_location_title(),
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
