import './style.scss';

import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate, useParams } from '@tanstack/react-router';
import { cloneDeep, omit } from 'lodash-es';
import { useMemo, useState } from 'react';
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
import { InfoBanner } from '../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { getLocationQueryOptions } from '../../shared/query';
import { validateIpList, validateIpOrDomainList } from '../../shared/validators';

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

const formSchema = z.object({
  name: z.string(m.form_error_required()).min(1, m.form_error_required()),
  address: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required())
    .refine((value) => validateIpList(value, ',', true), m.form_error_invalid()),
  endpoint: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  port: z.number(m.form_error_required()).max(65535, m.form_error_port_max()),
  allowed_ips: z.string(m.form_error_required()).trim(),
  dns: z
    .string()
    .trim()
    .nullable()
    .refine((val) => {
      if (!val) return true;
      return validateIpOrDomainList(val, ',', true, true);
    }),
  peer_disconnect_threshold: z.number(m.form_error_required()),
  keepalive_interval: z
    .number(m.form_error_required())
    .max(65535, m.form_error_port_max()),
  mtu: z.number(m.form_error_required()).min(72).max(0xffffffff),
  fwmark: z.number(m.form_error_required()).min(0).max(0xffffffff),
  allowed_groups: z.array(
    z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  ),
  location_mfa_mode: z.enum(LocationMfaMode),
  service_location_mode: z.enum(LocationServiceMode),
  firewall: z.enum(LocationFirewall),
});

type FormFields = z.infer<typeof formSchema>;

const EditLocationForm = ({ location }: { location: NetworkLocation }) => {
  const [allGroupsToggle, setAllGroupsToggle] = useState(
    location.allowed_groups.length === 0,
  );

  const navigate = useNavigate();

  const { data: groupsOptions } = useQuery({
    queryFn: api.group.getGroups,
    queryKey: ['group'],
    select: (resp) =>
      resp.data.groups.map(
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
  });

  const { mutate: deleteLocation, isPending: deletePending } = useMutation({
    mutationFn: () => api.location.deleteLocation(location.id),
    meta: {
      invalidate: ['network'],
    },
    onSuccess: () => {
      navigate({
        to: '/locations',
        replace: true,
      });
    },
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      name: location.name,
      address: location.address.join(', '),
      allowed_groups: location.allowed_groups,
      allowed_ips: location.allowed_ips.join(', '),
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
      await editLocation({
        id: location.id,
        data: {
          ...omit(clone, ['firewall']),
          acl_default_allow: clone.firewall === LocationFirewall.Allow,
          acl_enabled: !(clone.firewall === LocationFirewall.Disabled),
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
        <EditPageFormSection label="Public facing data">
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Location name" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="port">
            {(field) => <field.FormInput required label="Gateway port" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="endpoint">
            {(field) => (
              <field.FormInput required label="Gateway IP address or domain name" />
            )}
          </form.AppField>
        </EditPageFormSection>
        <EditPageFormSection label="Internal VPN settings">
          <form.AppField name="address">
            {(field) => (
              <field.FormInput required label="Gateway VPN IP address and netmask" />
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
              <field.FormInput required label="Keep alive interval (seconds)" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="mtu">
            {(field) => (
              <field.FormInput label="Maximum Transmission Unit (MTU)" type="number" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="fwmark">
            {(field) => <field.FormInput label="Firewall Mark (FwMark)" type="number" />}
          </form.AppField>
        </EditPageFormSection>
        <form.Subscribe
          selector={(s) =>
            s.values.service_location_mode !== LocationServiceMode.Disabled
          }
        >
          {(disabled) => (
            <form.AppField
              name="location_mfa_mode"
              validators={{
                onChangeListenTo: ['service_location_mode'],
              }}
              listeners={{
                onChange: ({ value, fieldApi }) => {
                  const service = fieldApi.form.getFieldValue('service_location_mode');
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
              {(field) => {
                return (
                  <>
                    {disabled && (
                      <InfoBanner
                        icon="info-outlined"
                        variant="warning"
                        text={`You can't use MFA on any service locations. If you want to enforce MAF please select “Regular location” type`}
                      />
                    )}
                    <EditPageFormSection label="Multi-Factor Authentication">
                      <field.FormRadio
                        value={LocationMfaMode.Disabled}
                        text="Do not enforce MFA"
                        disabled={disabled}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationMfaMode.Internal}
                        text="Internal MFA"
                        disabled={disabled}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationMfaMode.External}
                        text="External MFA"
                        disabled={disabled}
                      />
                    </EditPageFormSection>
                  </>
                );
              }}
            </form.AppField>
          )}
        </form.Subscribe>
        <form.Subscribe
          selector={(s) => s.values.location_mfa_mode !== LocationMfaMode.Disabled}
        >
          {(disabled) => (
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
                    {disabled && (
                      <InfoBanner
                        variant="warning"
                        icon="info-outlined"
                        text={
                          "If your location is MFA protected, you won't be able to set is as a service location. The location must have MFA disabled in order to use service location mode.You can read more about service locations in our documentation."
                        }
                      />
                    )}
                    <EditPageFormSection label="Location type (Windows only)">
                      <field.FormRadio
                        value={LocationServiceMode.Disabled}
                        text="Regular location"
                        disabled={disabled}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationServiceMode.Prelogon}
                        text="Service location: Pre-logon"
                        disabled={disabled}
                      />
                      <SizedBox height={ThemeSpacing.Md} />
                      <field.FormRadio
                        value={LocationServiceMode.Alwayson}
                        text="Service location: Always on"
                        disabled={disabled}
                      />
                    </EditPageFormSection>
                  </>
                );
              }}
            </form.AppField>
          )}
        </form.Subscribe>
        <EditPageFormSection label="Location Access">
          {isPresent(groupsOptions) && (
            <form.AppField name="allowed_groups">
              {(field) => (
                <field.FormSelectMultiple
                  options={groupsOptions}
                  counterText={(count) => `+${count} groups`}
                  editText="Edit groups"
                  modalTitle="Select allowed groups"
                  toggleText="All groups have access"
                  toggleValue={allGroupsToggle}
                  onToggleChange={(value) => {
                    setAllGroupsToggle(value);
                    if (value) {
                      field.handleChange([]);
                    }
                  }}
                />
              )}
            </form.AppField>
          )}
        </EditPageFormSection>
        <EditPageFormSection label="Firewall">
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Disabled}
                text="Disable firewall option"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Allow}
                text="Users/devices can access all resources unless limited by ACL rules."
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Md} />
          <form.AppField name="firewall">
            {(field) => (
              <field.FormRadio
                value={LocationFirewall.Deny}
                text="All traffic not explicitly allowed by an ACL rule will be blocked."
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
                  deleteLocation();
                },
                loading: deletePending,
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
