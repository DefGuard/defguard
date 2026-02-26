import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useRouter } from '@tanstack/react-router';
import { intersection } from 'lodash-es';
import { cloneDeep, omit } from 'radashi';
import { useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { AclRule } from '../../shared/api/types';
import { Controls } from '../../shared/components/Controls/Controls';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import type { SelectionOption } from '../../shared/components/SelectionSection/type';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import {
  getAppliedAliasesQueryOptions,
  getAppliedDestinationsQueryOptions,
  getGroupsInfoQueryOptions,
  getLocationsQueryOptions,
  getNetworkDevicesQueryOptions,
  getUsersOverviewQueryOptions,
} from '../../shared/query';
import { aclDestinationValidator, aclPortsValidator } from '../../shared/validators';
import aliasesEmptyImage from './assets/aliases-empty-icon.png';
import emptyDestinationIconSrc from './assets/empty-destinations-icon.png';
import { DestinationSection } from './components/DestinationSection';
import { GeneralSettingsSection } from './components/GeneralSettingsSection';
import { PermissionsSection } from './components/PermissionsSection';
import { RestrictionsSection } from './components/RestrictionsSection';
import type { CERuleFormValues } from './types';

type Props = {
  rule?: AclRule;
};

export const CERulePage = ({ rule }: Props) => {
  const isEdit = isPresent(rule);

  return (
    <EditPage
      id="ce-rule-page"
      pageTitle="Rules"
      headerProps={{
        icon: 'add-rule',
        title: isEdit ? `Edit firewall rule` : `Create rule for firewall`,
        subtitle: `Define who can access specific locations and which IPs, ports, and protocols are allowed. Use firewall rules to grant or restrict access for users and groups, ensuring your network stays secure and controlled.`,
      }}
    >
      <Content rule={rule} />
    </EditPage>
  );
};

const Content = ({ rule: initialRule }: Props) => {
  const router = useRouter();

  const isEdit = isPresent(initialRule);

  const { mutateAsync: addRule } = useMutation({
    mutationFn: api.acl.rule.addRule,
    meta: {
      invalidate: ['acl'],
    },
    onSuccess: () => {
      Snackbar.success('Rule added');
      router.history.back();
    },
  });

  const { mutateAsync: editRule } = useMutation({
    mutationFn: api.acl.rule.editRule,
    meta: {
      invalidate: ['acl'],
    },
    onSuccess: () => {
      Snackbar.success('Rule changed');
      router.history.back();
    },
  });

  const { data: users } = useQuery(getUsersOverviewQueryOptions);

  const usersOptions = useMemo(() => {
    if (isPresent(users)) {
      return users.map(
        (user): SelectionOption<number> => ({
          id: user.id,
          label: user.username,
          meta: user,
          searchFields: [user.username, user.email, user.first_name, user.last_name],
        }),
        [],
      );
    }
  }, [users]);

  const { data: destinations } = useQuery(getAppliedDestinationsQueryOptions);

  const destinationsOptions = useMemo(() => {
    if (isPresent(destinations)) {
      return destinations.map(
        (destination): SelectionOption<number> => ({
          id: destination.id,
          label: destination.name,
          meta: destination,
        }),
      );
    }
  }, [destinations]);

  const { data: locations } = useQuery(getLocationsQueryOptions);

  const locationsOptions = useMemo(() => {
    if (isPresent(locations)) {
      return locations.map(
        (location): SelectionOption<number> => ({
          id: location.id,
          label: location.name,
          meta: location,
        }),
      );
    }
    return [];
  }, [locations]);

  const { data: aliases } = useQuery(getAppliedAliasesQueryOptions);

  const aliasesOptions = useMemo(() => {
    if (isPresent(aliases)) {
      return aliases.map(
        (alias): SelectionOption<number> => ({
          id: alias.id,
          label: alias.name,
          meta: alias,
        }),
        [],
      );
    }
    return [];
  }, [aliases]);

  const { data: groups } = useQuery(getGroupsInfoQueryOptions);
  const groupsOptions = useMemo(() => {
    if (isPresent(groups)) {
      return groups.map(
        (group): SelectionOption<number> => ({
          id: group.id,
          label: group.name,
          meta: group,
        }),
      );
    }
    return [];
  }, [groups]);

  const { data: networkDevices } = useQuery(getNetworkDevicesQueryOptions);
  const networkDevicesOptions = useMemo(() => {
    if (isPresent(networkDevices)) {
      return networkDevices.map(
        (device): SelectionOption<number> => ({
          id: device.id,
          label: device.name,
          meta: device,
        }),
      );
    }
    return [];
  }, [networkDevices]);

  const [restrictUsers, setRestrictUsers] = useState(() =>
    isPresent(initialRule)
      ? initialRule.deny_all_users || initialRule.denied_users.length > 0
      : false,
  );
  const [restrictGroups, setRestrictGroups] = useState(() =>
    isPresent(initialRule)
      ? initialRule.deny_all_groups || initialRule.denied_groups.length > 0
      : false,
  );
  const [restrictDevices, setRestrictDevices] = useState(() =>
    isPresent(initialRule)
      ? initialRule.deny_all_network_devices ||
        initialRule.denied_network_devices.length > 0
      : false,
  );

  const formSchema = useMemo(
    () =>
      z
        .object({
          name: z.string(m.form_error_required()).min(1, m.form_error_required()),
          locations: z.number().array(),
          expires: z.string().nullable(),
          enabled: z.boolean(),
          all_locations: z.boolean(),
          allow_all_users: z.boolean(),
          deny_all_users: z.boolean(),
          allow_all_groups: z.boolean(),
          deny_all_groups: z.boolean(),
          allow_all_network_devices: z.boolean(),
          deny_all_network_devices: z.boolean(),
          allowed_users: z.number().array(),
          denied_users: z.number().array(),
          allowed_groups: z.number().array(),
          denied_groups: z.number().array(),
          allowed_network_devices: z.number().array(),
          denied_network_devices: z.number().array(),
          addresses: aclDestinationValidator,
          ports: aclPortsValidator,
          protocols: z.set(z.number()),
          any_address: z.boolean(),
          any_port: z.boolean(),
          any_protocol: z.boolean(),
          destinations: z.set(z.number()),
          aliases: z.set(z.number()),
          use_manual_destination_settings: z.boolean(),
        })
        .superRefine((vals, ctx) => {
          // check for collisions
          // FIXME: add handling for all_groups toggles
          const message = 'Allow Deny conflict error placeholder.';
          if (!vals.allow_all_users && !vals.deny_all_users) {
            if (intersection(vals.allowed_users, vals.denied_users).length) {
              ctx.addIssue({
                path: ['allowed_users'],
                code: 'custom',
                message,
              });
              ctx.addIssue({
                path: ['denied_users'],
                code: 'custom',
                message,
              });
            }
            if (intersection(vals.allowed_groups, vals.denied_groups).length) {
              ctx.addIssue({
                path: ['allowed_groups'],
                code: 'custom',
                message,
              });
              ctx.addIssue({
                path: ['denied_groups'],
                code: 'custom',
                message,
              });
            }
          }
          if (!vals.allow_all_network_devices && !vals.deny_all_network_devices) {
            if (
              intersection(vals.allowed_network_devices, vals.denied_network_devices)
                .length
            ) {
              ctx.addIssue({
                path: ['allowed_network_devices'],
                code: 'custom',
                message,
              });
              ctx.addIssue({
                path: ['denied_network_devices'],
                code: 'custom',
                message,
              });
            }
          }

          // check if one of allowed users/groups/devices fields is set
          const isAllowConfigured =
            vals.allow_all_users ||
            vals.allow_all_groups ||
            vals.allow_all_network_devices ||
            vals.allowed_users.length !== 0 ||
            vals.allowed_groups.length !== 0 ||
            vals.allowed_network_devices.length !== 0;
          if (!isAllowConfigured) {
            const message = 'Must configure some allowed users, groups or devices';
            ctx.addIssue({
              path: ['allowed_users'],
              code: 'custom',
              message,
            });
            ctx.addIssue({
              path: ['allowed_groups'],
              code: 'custom',
              message,
            });
            ctx.addIssue({
              path: ['allowed_network_devices'],
              code: 'custom',
              message,
            });
          }
        }),
    [],
  );

  type FormFields = CERuleFormValues;

  const defaultValues = useMemo((): FormFields => {
    if (isPresent(initialRule)) {
      return {
        ...omit(initialRule, ['id', 'state', 'expires', 'parent_id']),
        aliases: new Set(initialRule.aliases),
        destinations: new Set(initialRule.destinations),
        protocols: new Set(initialRule.protocols),
        expires: null,
      };
    }

    return {
      name: '',
      addresses: '',
      ports: '',
      aliases: new Set(),
      destinations: new Set(),
      allowed_network_devices: [],
      allowed_groups: [],
      allowed_users: [],
      denied_network_devices: [],
      denied_groups: [],
      denied_users: [],
      locations: [],
      protocols: new Set(),
      all_locations: true,
      allow_all_users: true,
      allow_all_groups: true,
      allow_all_network_devices: true,
      deny_all_users: false,
      deny_all_groups: false,
      deny_all_network_devices: false,
      enabled: true,
      expires: null,
      any_address: true,
      any_port: true,
      any_protocol: true,
      use_manual_destination_settings: false,
    };
  }, [initialRule]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      const toSend = cloneDeep(value);
      if (!restrictUsers) {
        toSend.deny_all_users = false;
        toSend.denied_users = [];
      }
      if (!restrictGroups) {
        toSend.deny_all_groups = false;
        toSend.denied_groups = [];
      }
      if (!restrictDevices) {
        toSend.deny_all_network_devices = false;
        toSend.denied_network_devices = [];
      }
      if (isPresent(initialRule)) {
        await editRule({
          ...toSend,
          protocols: Array.from(toSend.protocols),
          aliases: Array.from(toSend.aliases),
          destinations: Array.from(toSend.destinations),
          id: initialRule.id,
        });
      } else {
        await addRule({
          ...toSend,
          protocols: Array.from(toSend.protocols),
          aliases: Array.from(toSend.aliases),
          destinations: Array.from(toSend.destinations),
        });
      }
    },
  });

  const selectedAliases = useStore(
    form.store,
    (s) => aliases?.filter((alias) => s.values.aliases.has(alias.id)) ?? [],
  );

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <GeneralSettingsSection form={form} locationsOptions={locationsOptions} />
        <Divider spacing={ThemeSpacing.Xl2} />
        <DestinationSection
          form={form}
          destinations={destinations}
          destinationsOptions={destinationsOptions}
          aliasesOptions={aliasesOptions}
          selectedAliases={selectedAliases}
          aliasesEmptyImage={aliasesEmptyImage}
          emptyDestinationIconSrc={emptyDestinationIconSrc}
        />
        <Divider spacing={ThemeSpacing.Xl2} />
        <PermissionsSection
          form={form}
          usersOptions={usersOptions}
          groupsOptions={groupsOptions}
          networkDevicesOptions={networkDevicesOptions}
        />
        <Divider spacing={ThemeSpacing.Xl2} />
        <RestrictionsSection
          form={form}
          usersOptions={usersOptions}
          groupsOptions={groupsOptions}
          networkDevicesOptions={networkDevicesOptions}
          restrictUsers={restrictUsers}
          restrictGroups={restrictGroups}
          restrictDevices={restrictDevices}
          setRestrictUsers={setRestrictUsers}
          setRestrictGroups={setRestrictGroups}
          setRestrictDevices={setRestrictDevices}
        />
        <Divider spacing={ThemeSpacing.Xl2} />
        <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
          {({ isSubmitting }) => (
            <Controls>
              <form.AppField name="enabled">
                {(field) => <field.FormToggle label="Enable rule" />}
              </form.AppField>
              <div className="right">
                <Button
                  text={isEdit ? 'Save changes' : 'Create rule'}
                  type="submit"
                  loading={isSubmitting}
                />
              </div>
            </Controls>
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
