import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useNavigate, useRouter } from '@tanstack/react-router';
import { intersection } from 'lodash-es';
import { cloneDeep, flat, omit } from 'radashi';
import { useCallback, useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import type { AclListTabValue } from '../../shared/aclTabs';
import api from '../../shared/api/api';
import {
  type AclDestination,
  AclProtocolName,
  type AclProtocolValue,
  type AclRule,
  aclProtocolValues,
  type NetworkLocation,
} from '../../shared/api/types';
import { Card } from '../../shared/components/Card/Card';
import { Controls } from '../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../shared/components/DescriptionBlock/DescriptionBlock';
import { DestinationDismissibleBox } from '../../shared/components/DestinationDismissibleBox/DestinationDismissibleBox';
import { DestinationLabel } from '../../shared/components/DestinationLabel/DestinationLabel';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { useSelectionModal } from '../../shared/components/modals/SelectionModal/useSelectionModal';
import type {
  SelectionOption,
  SelectionSectionCustomRender,
} from '../../shared/components/SelectionSection/type';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { ButtonsGroup } from '../../shared/defguard-ui/components/ButtonsGroup/ButtonsGroup';
import { Checkbox } from '../../shared/defguard-ui/components/Checkbox/Checkbox';
import { CheckboxIndicator } from '../../shared/defguard-ui/components/CheckboxIndicator/CheckboxIndicator';
import { Chip } from '../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { FieldError } from '../../shared/defguard-ui/components/FieldError/FieldError';
import { Fold } from '../../shared/defguard-ui/components/Fold/Fold';
import { Icon, type IconKindValue } from '../../shared/defguard-ui/components/Icon';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { useFormFieldError } from '../../shared/defguard-ui/hooks/useFormFieldError';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { TooltipContent } from '../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { openModal } from '../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../shared/hooks/modalControls/modalTypes';
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

const getProtocolName = (key: AclProtocolValue) => AclProtocolName[key];

const getSelectedLocationsCounterText = (count: number) => {
  if (count === 1) {
    return m.acl_rule_selected_location_count_one({ count });
  }

  return m.acl_rule_selected_location_count_other({ count });
};

const getSelectedUsersCounterText = (count: number) => {
  if (count === 1) {
    return m.acl_rule_selected_user_count_one({ count });
  }

  return m.acl_rule_selected_user_count_other({ count });
};

const getSelectedGroupsCounterText = (count: number) => {
  if (count === 1) {
    return m.location_access_selected_group_count_one({ count });
  }

  return m.location_access_selected_group_count_other({ count });
};

const getSelectedNetworkDevicesCounterText = (count: number) => {
  if (count === 1) {
    return m.acl_rule_selected_network_device_count_one({ count });
  }

  return m.acl_rule_selected_network_device_count_other({ count });
};

const renderDestinationSelectionItem: SelectionSectionCustomRender<
  number,
  AclDestination
> = ({ active, onClick, option }) => (
  <div className="destination-selection-item" onClick={onClick}>
    <CheckboxIndicator active={active} />
    {isPresent(option.meta) && (
      <DestinationLabel
        name={option.meta.name}
        ips={option.meta.addresses}
        ports={option.meta.ports}
        protocols={option.meta.protocols
          .map((protocol) => AclProtocolName[protocol])
          .join(',')}
      />
    )}
  </div>
);

const renderLocationSelectionItem: SelectionSectionCustomRender<
  number,
  NetworkLocation
> = ({ active, onClick, option }) => {
  const icon: IconKindValue = 'check';
  return (
    <div className="item location-selection-item" onClick={onClick}>
      <CheckboxIndicator active={active} />
      {isPresent(option.meta) && (
        <>
          <div className="content-track">
            <p className="item-label">{option.meta?.name}</p>
          </div>
          <TooltipProvider>
            <TooltipTrigger>
              <Icon icon={icon} size={16} />
            </TooltipTrigger>
            <TooltipContent>
              {!option.meta.acl_enabled && (
                <p>{m.acl_rule_location_access_unmanaged()}</p>
              )}
              {option.meta.acl_enabled && option.meta.acl_default_allow && (
                <p>{m.acl_rule_location_access_default_allow()}</p>
              )}
              {option.meta.acl_enabled && !option.meta.acl_default_allow && (
                <p>{m.acl_rule_location_access_default_deny()}</p>
              )}
            </TooltipContent>
          </TooltipProvider>
        </>
      )}
    </div>
  );
};

type Props = {
  rule?: AclRule;
  tab?: AclListTabValue;
};

export const CERulePage = ({ rule, tab }: Props) => {
  const isEdit = isPresent(rule);

  return (
    <EditPage
      id="ce-rule-page"
      pageTitle={m.cmp_nav_item_rules()}
      headerProps={{
        icon: 'add-rule',
        title: isEdit ? m.acl_rule_form_title_edit() : m.acl_rule_form_title_add(),
        subtitle: m.acl_rule_form_subtitle(),
      }}
    >
      <Content rule={rule} tab={tab} />
    </EditPage>
  );
};

const Content = ({ rule: initialRule, tab }: Props) => {
  const router = useRouter();
  const navigate = useNavigate();

  const isEdit = isPresent(initialRule);
  const returnToRules = useCallback(() => {
    if (tab === undefined) {
      router.history.back();
      return;
    }

    navigate({
      to: '/acl/rules',
      search: {
        tab,
      },
    });
  }, [navigate, router, tab]);

  const { mutateAsync: addRule } = useMutation({
    mutationFn: api.acl.rule.addRule,
    meta: {
      invalidate: ['acl'],
    },
    onSuccess: () => {
      Snackbar.default(m.acl_rule_submit_success());
      returnToRules();
    },
  });

  const { mutateAsync: editRule } = useMutation({
    mutationFn: api.acl.rule.editRule,
    meta: {
      invalidate: ['acl'],
    },
    onSuccess: () => {
      Snackbar.default(m.acl_rule_submit_success());
      returnToRules();
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
  const hasPredefinedDestinations = Boolean(destinations && destinations.length > 0);

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
          const message = m.acl_rule_error_allow_deny_conflict();
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

          if (restrictUsers && !vals.deny_all_users && vals.denied_users.length === 0) {
            ctx.addIssue({
              path: ['denied_users'],
              code: 'custom',
              message: m.form_select_at_least_one(),
            });
          }

          if (
            restrictGroups &&
            !vals.deny_all_groups &&
            vals.denied_groups.length === 0
          ) {
            ctx.addIssue({
              path: ['denied_groups'],
              code: 'custom',
              message: m.form_select_at_least_one(),
            });
          }

          if (
            restrictDevices &&
            !vals.deny_all_network_devices &&
            vals.denied_network_devices.length === 0
          ) {
            ctx.addIssue({
              path: ['denied_network_devices'],
              code: 'custom',
              message: m.form_select_at_least_one(),
            });
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
            const message = m.acl_rule_error_permissions_required();
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

          if (vals.use_manual_destination_settings) {
            const message = m.acl_rule_error_manual_destination_required();
            if (!vals.any_address && vals.addresses.trim().length === 0) {
              ctx.addIssue({
                path: ['addresses'],
                code: 'custom',
                message,
              });
            }
            if (!vals.any_port && vals.ports.trim().length === 0) {
              ctx.addIssue({
                path: ['ports'],
                code: 'custom',
                message,
              });
            }
            if (!vals.any_protocol && vals.protocols.size === 0) {
              ctx.addIssue({
                path: ['protocols'],
                code: 'custom',
                message,
              });
            }
          } else if (vals.destinations.size === 0) {
            // If no predefined destinations exist, show error under the "Add manual destination settings" checkbox.
            // If predefined destinations exist - show it at the end of "Destination" section.
            ctx.addIssue({
              path: [
                hasPredefinedDestinations
                  ? 'destinations'
                  : 'use_manual_destination_settings',
              ],
              code: 'custom',
              message: hasPredefinedDestinations
                ? m.form_error_no_destination()
                : m.form_error_no_predefined_destination(),
            });
          }
        }),
    [hasPredefinedDestinations, restrictDevices, restrictGroups, restrictUsers],
  );

  type FormFields = z.infer<typeof formSchema>;

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
        <MarkedSection icon="settings">
          <AppText font={TextStyle.TBodyPrimary600}>
            {m.acl_rule_section_general_settings()}
          </AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="name">
            {(field) => (
              <field.FormInput
                required
                label={m.acl_rules_col_name()}
                helper={m.acl_helper_rule_name()}
              />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title={m.cmp_nav_item_locations()}>
            <p>{m.acl_rule_locations_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.Subscribe selector={(s) => s.values.all_locations}>
            {(allValue) => (
              <form.AppField name="locations">
                {(field) => (
                  <field.FormSelectMultiple
                    options={locationsOptions}
                    counterText={getSelectedLocationsCounterText}
                    editText={m.acl_rule_edit_locations()}
                    modalTitle={m.acl_rule_select_locations()}
                    toggleText={m.acl_rule_include_all_locations()}
                    selectionCustomItemRender={renderLocationSelectionItem}
                    toggleValue={allValue}
                    onToggleChange={(value) => {
                      form.setFieldValue('all_locations', value);
                    }}
                  />
                )}
              </form.AppField>
            )}
          </form.Subscribe>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="location-tracking">
          <AppText font={TextStyle.TBodyPrimary600}>
            {m.acl_rule_section_destination()}
          </AppText>
          <SizedBox height={ThemeSpacing.Sm} />
          <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
            {m.acl_rule_destinations_description()}
          </AppText>
          <SizedBox height={ThemeSpacing.Xl2} />
          {isPresent(destinations) && destinations.length === 0 && (
            <div className="no-resource">
              <div className="icon-box">
                <img src={emptyDestinationIconSrc} height={40} width={41} />
              </div>
              <p>{m.acl_rule_no_predefined_destinations()}</p>
            </div>
          )}
          {isPresent(destinations) && destinations.length > 0 && (
            <form.AppField name="destinations">
              {(field) => {
                const selectedDestinations =
                  destinations?.filter((destination) =>
                    field.state.value.has(destination.id),
                  ) ?? [];
                return (
                  <>
                    <Button
                      variant="outlined"
                      text={m.acl_rule_select_predefined_destinations()}
                      onClick={() => {
                        useSelectionModal.setState({
                          title: m.acl_rule_select_predefined_destinations(),
                          isOpen: true,
                          options: destinationsOptions ?? [],
                          itemGap: 12,
                          enableDividers: true,
                          onSubmit: (selection) =>
                            field.handleChange(new Set(selection as number[])),
                          // @ts-expect-error
                          renderItem: renderDestinationSelectionItem,
                        });
                      }}
                    />
                    {selectedDestinations.length > 0 && (
                      <div className="selected-destinations">
                        <div className="top">
                          <p>{m.acl_rule_selected_destinations()}</p>
                        </div>
                        <div className="items-track">
                          {selectedDestinations.map((destination) => (
                            <DestinationDismissibleBox
                              key={destination.id}
                              name={destination.name}
                              ips={destination.addresses}
                              ports={destination.ports}
                              protocols={destination.protocols
                                .map((p) => AclProtocolName[p])
                                .join(',')}
                              onClick={() => {
                                const newValue = new Set(field.state.value);
                                newValue.delete(destination.id);
                                field.handleChange(newValue);
                              }}
                            />
                          ))}
                        </div>
                      </div>
                    )}
                  </>
                );
              }}
            </form.AppField>
          )}
          <Divider text={`${m.misc_or()}/${m.misc_and()}`} spacing={ThemeSpacing.Lg} />
          <DescriptionBlock title={m.acl_rule_define_destination_manually()}>
            <p>{m.acl_rule_manual_destination_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="use_manual_destination_settings">
            {(field) => (
              <field.FormCheckbox text={m.acl_rule_add_manual_destination_settings()} />
            )}
          </form.AppField>
          <form.Subscribe selector={(s) => s.values.use_manual_destination_settings}>
            {(open) => (
              <Fold open={open}>
                <SizedBox height={ThemeSpacing.Xl2} />
                <Card>
                  {isPresent(aliasesOptions) && aliasesOptions.length === 0 && (
                    <div className="no-resource">
                      <div className="icon-box">
                        <img src={aliasesEmptyImage} height={40} />
                      </div>
                      <p>{m.acl_rule_no_aliases()}</p>
                    </div>
                  )}
                  {isPresent(aliasesOptions) && aliasesOptions.length > 0 && (
                    <>
                      <DescriptionBlock title={m.cmp_nav_item_aliases()}>
                        <p>{m.acl_rule_aliases_description()}</p>
                      </DescriptionBlock>
                      <SizedBox height={ThemeSpacing.Lg} />
                      <form.AppField name="aliases">
                        {(field) => (
                          <>
                            <ButtonsGroup>
                              <Button
                                variant="outlined"
                                text={m.acl_rule_apply_aliases()}
                                disabled={aliasesOptions?.length === 0}
                                onClick={() => {
                                  useSelectionModal.setState({
                                    isOpen: true,
                                    onSubmit: (selected) => {
                                      field.handleChange(new Set(selected as number[]));
                                    },
                                    options: aliasesOptions,
                                    selected: new Set(field.state.value),
                                    title: m.acl_rule_select_aliases(),
                                  });
                                }}
                              />
                            </ButtonsGroup>
                            <SizedBox height={ThemeSpacing.Xl} />
                            {isPresent(aliasesOptions) && (
                              <div className="aliases-selected">
                                {aliasesOptions
                                  .filter((alias) => field.state.value.has(alias.id))
                                  .map((option) => (
                                    <Chip
                                      size="sm"
                                      text={option.label}
                                      key={option.id}
                                      onDismiss={() => {
                                        const newState = new Set(field.state.value);
                                        newState.delete(option.id);
                                        field.handleChange(newState);
                                      }}
                                    />
                                  ))}
                              </div>
                            )}
                          </>
                        )}
                      </form.AppField>
                    </>
                  )}
                  <Divider spacing={ThemeSpacing.Xl} />
                  <DescriptionBlock title={m.acl_form_section_addresses_title()}>
                    <p>{m.acl_form_section_addresses_description()}</p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="any_address">
                    {(field) => (
                      <field.FormToggle label={m.acl_destination_any_address()} />
                    )}
                  </form.AppField>
                  <form.Subscribe selector={(s) => !s.values.any_address}>
                    {(open) => (
                      <Fold open={open}>
                        <SizedBox height={ThemeSpacing.Xl} />
                        <form.AppField name="addresses">
                          {(field) => (
                            <field.FormTextarea
                              label={m.acl_form_addresses_label()}
                              helper={m.acl_helper_addresses()}
                            />
                          )}
                        </form.AppField>
                        <AliasDataBlock
                          values={flat(
                            selectedAliases.map((alias) => alias.addresses.split(',')),
                          )}
                        />
                      </Fold>
                    )}
                  </form.Subscribe>
                  <Divider spacing={ThemeSpacing.Xl} />
                  <DescriptionBlock title={m.acl_form_section_ports_title()}>
                    <p>{m.acl_form_section_ports_description()}</p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="any_port">
                    {(field) => <field.FormToggle label={m.acl_destination_any_port()} />}
                  </form.AppField>
                  <form.Subscribe selector={(s) => !s.values.any_port}>
                    {(open) => (
                      <Fold open={open}>
                        <SizedBox height={ThemeSpacing.Xl} />
                        <form.AppField name="ports">
                          {(field) => (
                            <field.FormInput
                              label={m.acl_form_ports_label()}
                              helper={m.acl_helper_ports()}
                            />
                          )}
                        </form.AppField>
                        <AliasDataBlock
                          values={flat(
                            selectedAliases.map((alias) => alias.ports.split(',')),
                          )}
                        />
                      </Fold>
                    )}
                  </form.Subscribe>
                  <Divider spacing={ThemeSpacing.Xl} />
                  <DescriptionBlock title={m.acl_form_section_protocols_title()}>
                    <p>{m.acl_form_section_protocols_description()}</p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="any_protocol">
                    {(field) => (
                      <field.FormToggle label={m.acl_destination_any_protocol()} />
                    )}
                  </form.AppField>
                  <form.Subscribe selector={(s) => !s.values.any_protocol}>
                    {(open) => (
                      <Fold open={open}>
                        <SizedBox height={ThemeSpacing.Xl2} />
                        <form.AppField name="protocols">
                          {(field) => (
                            <field.FormCheckboxGroup
                              values={aclProtocolValues}
                              getLabel={getProtocolName}
                            />
                          )}
                        </form.AppField>
                        <AliasDataBlock
                          values={flat(
                            selectedAliases.map((alias) =>
                              alias.protocols.map(
                                (protocol) => AclProtocolName[protocol],
                              ),
                            ),
                          )}
                        />
                      </Fold>
                    )}
                  </form.Subscribe>
                </Card>
              </Fold>
            )}
          </form.Subscribe>
          <form.AppField name="destinations">
            {() => (
              <DestinationSelectionError
                hasPredefinedDestinations={hasPredefinedDestinations}
              />
            )}
          </form.AppField>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="enrollment">
          <AppText font={TextStyle.TBodyPrimary600}>
            {m.acl_rule_section_permissions()}
          </AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title={m.acl_rule_permissions_description_title()}>
            <p>{m.acl_rule_permissions_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          {isPresent(usersOptions) && (
            <form.Subscribe selector={(s) => s.values.allow_all_users}>
              {(allowAllValue) => (
                <form.AppField name="allowed_users">
                  {(field) => (
                    <field.FormSelectMultiple
                      toggleValue={allowAllValue}
                      toggleText={m.acl_rule_all_users_have_access()}
                      counterText={getSelectedUsersCounterText}
                      editText={m.acl_rule_edit_users()}
                      modalTitle={m.acl_rule_select_allowed_users()}
                      options={usersOptions}
                      onToggleChange={(value) => {
                        form.setFieldValue('allow_all_users', value);
                      }}
                    />
                  )}
                </form.AppField>
              )}
            </form.Subscribe>
          )}
          <Divider spacing={ThemeSpacing.Lg} />
          {isPresent(groupsOptions) && (
            <form.Subscribe selector={(s) => s.values.allow_all_groups}>
              {(allAllowedValue) => (
                <form.AppField name="allowed_groups">
                  {(field) => (
                    <field.FormSelectMultiple
                      toggleValue={allAllowedValue}
                      onToggleChange={(value) => {
                        form.setFieldValue('allow_all_groups', value);
                      }}
                      options={groupsOptions}
                      counterText={getSelectedGroupsCounterText}
                      editText={m.location_access_edit_groups()}
                      modalTitle={m.location_access_select_allowed_groups()}
                      toggleText={m.location_access_all_groups_have_access()}
                    />
                  )}
                </form.AppField>
              )}
            </form.Subscribe>
          )}
          <Divider spacing={ThemeSpacing.Lg} />
          {isPresent(networkDevicesOptions) && (
            <form.Subscribe selector={(s) => s.values.allow_all_network_devices}>
              {(allowAllValue) => (
                <form.AppField name="allowed_network_devices">
                  {(field) => (
                    <field.FormSelectMultiple
                      toggleValue={allowAllValue}
                      onToggleChange={(value) => {
                        form.setFieldValue('allow_all_network_devices', value);
                      }}
                      options={networkDevicesOptions}
                      counterText={getSelectedNetworkDevicesCounterText}
                      editText={m.acl_rule_edit_network_devices()}
                      modalTitle={m.acl_rule_select_allowed_network_devices()}
                      toggleText={m.acl_rule_all_network_devices_have_access()}
                    />
                  )}
                </form.AppField>
              )}
            </form.Subscribe>
          )}
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="lock-closed">
          <AppText font={TextStyle.TBodyPrimary600}>
            {m.acl_rule_section_restrictions()}
          </AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title={m.acl_rule_limit_access()}>
            <p>{m.acl_rule_limit_access_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          {isPresent(usersOptions) && (
            <>
              <Checkbox
                active={restrictUsers}
                onClick={() => {
                  setRestrictUsers((current) => !current);
                }}
                text={m.acl_rule_limit_access_users()}
              />
              <Fold open={restrictUsers}>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="deny_all_users">
                  {(field) => (
                    <field.FormRadio text={m.acl_rule_exclude_all_users()} value={true} />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Md} />
                <form.AppField name="deny_all_users">
                  {(field) => (
                    <field.FormRadio
                      text={m.acl_rule_exclude_specific_users()}
                      value={false}
                    />
                  )}
                </form.AppField>
                <form.Subscribe
                  selector={(s) => s.values.deny_all_users === false && restrictUsers}
                >
                  {(open) => (
                    <Fold open={open}>
                      <SizedBox height={ThemeSpacing.Lg} />
                      {isPresent(usersOptions) && (
                        <form.AppField name="denied_users">
                          {(field) => (
                            <field.FormSelectMultiple
                              toggleValue={!open}
                              onToggleChange={() => {}}
                              counterText={getSelectedUsersCounterText}
                              editText={m.acl_rule_edit_users()}
                              modalTitle={m.acl_rule_select_restricted_users()}
                              options={usersOptions}
                            />
                          )}
                        </form.AppField>
                      )}
                    </Fold>
                  )}
                </form.Subscribe>
              </Fold>
            </>
          )}
          <Divider spacing={ThemeSpacing.Lg} />
          {isPresent(groupsOptions) && (
            <>
              <Checkbox
                active={restrictGroups}
                onClick={() => {
                  setRestrictGroups((current) => !current);
                }}
                text={m.acl_rule_limit_access_groups()}
              />
              <Fold open={restrictGroups}>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="deny_all_groups">
                  {(field) => (
                    <field.FormRadio
                      text={m.acl_rule_exclude_all_groups()}
                      value={true}
                    />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Md} />
                <form.AppField name="deny_all_groups">
                  {(field) => (
                    <field.FormRadio
                      text={m.acl_rule_exclude_specific_groups()}
                      value={false}
                    />
                  )}
                </form.AppField>
                <form.Subscribe
                  selector={(s) => s.values.deny_all_groups === false && restrictGroups}
                >
                  {(open) => (
                    <Fold open={open}>
                      <SizedBox height={ThemeSpacing.Lg} />
                      {isPresent(groupsOptions) && (
                        <form.AppField name="denied_groups">
                          {(field) => (
                            <field.FormSelectMultiple
                              toggleValue={!open}
                              onToggleChange={() => {}}
                              counterText={getSelectedGroupsCounterText}
                              editText={m.location_access_edit_groups()}
                              modalTitle={m.acl_rule_select_restricted_groups()}
                              options={groupsOptions}
                            />
                          )}
                        </form.AppField>
                      )}
                    </Fold>
                  )}
                </form.Subscribe>
              </Fold>
            </>
          )}
          <Divider spacing={ThemeSpacing.Lg} />
          {isPresent(networkDevicesOptions) && (
            <>
              <Checkbox
                active={restrictDevices}
                onClick={() => {
                  setRestrictDevices((current) => !current);
                }}
                text={m.acl_rule_limit_access_network_devices()}
              />
              <Fold open={restrictDevices}>
                <SizedBox height={ThemeSpacing.Xl2} />
                <form.AppField name="deny_all_network_devices">
                  {(field) => (
                    <field.FormRadio
                      text={m.acl_rule_exclude_all_network_devices()}
                      value={true}
                    />
                  )}
                </form.AppField>
                <SizedBox height={ThemeSpacing.Md} />
                <form.AppField name="deny_all_network_devices">
                  {(field) => (
                    <field.FormRadio
                      text={m.acl_rule_exclude_specific_network_devices()}
                      value={false}
                    />
                  )}
                </form.AppField>
                <form.Subscribe
                  selector={(s) =>
                    s.values.deny_all_network_devices === false && restrictDevices
                  }
                >
                  {(open) => (
                    <Fold open={open}>
                      <SizedBox height={ThemeSpacing.Lg} />
                      {isPresent(networkDevicesOptions) && (
                        <form.AppField name="denied_network_devices">
                          {(field) => (
                            <field.FormSelectMultiple
                              toggleValue={!open}
                              onToggleChange={() => {}}
                              counterText={getSelectedNetworkDevicesCounterText}
                              editText={m.acl_rule_edit_network_devices()}
                              modalTitle={m.acl_rule_select_restricted_network_devices()}
                              options={networkDevicesOptions}
                            />
                          )}
                        </form.AppField>
                      )}
                    </Fold>
                  )}
                </form.Subscribe>
              </Fold>
            </>
          )}
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <form.Subscribe selector={(s) => ({ isSubmitting: s.isSubmitting })}>
          {({ isSubmitting }) => (
            <Controls>
              <form.AppField name="enabled">
                {(field) => <field.FormToggle label={m.acl_rule_enable()} />}
              </form.AppField>
              <div className="right">
                <Button
                  text={m.controls_cancel()}
                  variant="secondary"
                  onClick={() => {
                    returnToRules();
                  }}
                />
                <Button
                  text={isEdit ? m.controls_save_changes() : m.acl_rule_action_create()}
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

type AliasDataBlockProps = {
  values: string[];
};

const normalizeAliasValues = (values: string[]) => {
  const seenValues = new Set<string>();

  return values.reduce<string[]>((normalizedValues, value) => {
    const trimmedValue = value.trim();

    if (trimmedValue.length === 0 || seenValues.has(trimmedValue)) {
      return normalizedValues;
    }

    seenValues.add(trimmedValue);
    normalizedValues.push(trimmedValue);
    return normalizedValues;
  }, []);
};

const AliasDataBlock = ({ values }: AliasDataBlockProps) => {
  const normalizedValues = normalizeAliasValues(values);

  if (normalizedValues.length === 0) return null;

  return (
    <div className="alias-data-block">
      <div className="top">
        <p>{m.acl_rule_data_from_aliases()}</p>
      </div>
      <div className="content-track">
        {normalizedValues.map((value) => (
          <Chip key={value} text={value} />
        ))}
        {normalizedValues.length > 4 && (
          <button
            type="button"
            onClick={() => {
              openModal(ModalName.DisplayList, {
                title: m.acl_rule_data_from_aliases(),
                data: normalizedValues,
              });
            }}
          >
            <span>{m.acl_rule_show_all_alias_data()}</span>
          </button>
        )}
      </div>
    </div>
  );
};

const DestinationSelectionError = ({
  hasPredefinedDestinations,
}: {
  hasPredefinedDestinations: boolean;
}) => {
  const error = useFormFieldError();

  if (!hasPredefinedDestinations || !error) return null;

  return <FieldError error={error} />;
};
