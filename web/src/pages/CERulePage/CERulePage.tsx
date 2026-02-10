import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useRouter } from '@tanstack/react-router';
import { intersection } from 'lodash-es';
import { cloneDeep, flat, omit } from 'radashi';
import { useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
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
import { CheckboxIndicator } from '../../shared/defguard-ui/components/CheckboxIndicator/CheckboxIndicator';
import { Chip } from '../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../shared/defguard-ui/components/Fold/Fold';
import { Icon, type IconKindValue } from '../../shared/defguard-ui/components/Icon';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
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
                <p>{`Location access unmanaged (ACL disabled)`}</p>
              )}
              {option.meta.acl_enabled && option.meta.acl_default_allow && (
                <p>{`Location access allowed by default - network traffic not explicitly defined by the rules will be passed.`}</p>
              )}
              {option.meta.acl_enabled && !option.meta.acl_default_allow && (
                <p>{`Location access denied by default - network traffic not explicitly defined by the rules will be blocked.`}</p>
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

  const [_restrictionsPresent, _setRestrictionsPresent] = useState(false);
  // const [manualDestination, setManualDestination] = useState(false);

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
      // FIXME: When restrictions section is reworked
      toSend.deny_all_network_devices = false;
      toSend.deny_all_users = false;
      toSend.deny_all_groups = false;
      toSend.denied_network_devices = [];
      toSend.denied_groups = [];
      toSend.denied_users = [];
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
          <AppText font={TextStyle.TBodyPrimary600}>{`General settings`}</AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Rule name" />}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl2} />
          <DescriptionBlock title="Locations">
            <p>{`Specify which locations this rule applies to. You can select all available locations or choose specific ones based on your requirements.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.Subscribe selector={(s) => s.values.all_locations}>
            {(allValue) => (
              <form.AppField name="locations">
                {(field) => (
                  <field.FormSelectMultiple
                    options={locationsOptions}
                    counterText={(counter) => `Locations ${counter}`}
                    editText="Edit locations"
                    modalTitle="Select locations"
                    toggleText="Include all locations"
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
          <AppText font={TextStyle.TBodyPrimary600}>{`Destination`}</AppText>
          <SizedBox height={ThemeSpacing.Sm} />
          <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
            {`You can add additional destinations to this rule to extend its scope. These destinations are configured separately in the 'Destinations' section.`}
          </AppText>
          <SizedBox height={ThemeSpacing.Xl2} />
          {isPresent(destinations) && destinations.length === 0 && (
            <div className="no-resource">
              <div className="icon-box">
                <img src={emptyDestinationIconSrc} height={40} width={41} />
              </div>
              <p>{`You don't have any predefined destinations yet — add them in the 'Destinations' section to create reusable elements for defining destinations across multiple firewall ACL rules.`}</p>
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
                      text="Select predefined destination(s)"
                      onClick={() => {
                        useSelectionModal.setState({
                          title: 'Select predefined destination(s)',
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
                          <p>{`Selected destinations`}</p>
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
          <Divider text="or/and" spacing={ThemeSpacing.Lg} />
          <DescriptionBlock title={`Define destination manually`}>
            <p>{`Manually configure destinations parameters for this rule.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="use_manual_destination_settings">
            {(field) => <field.FormCheckbox text="Add manual destination settings" />}
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
                      <p>{`You don't have any aliases to use yet — create them in the “Aliases” section to create reusable elements for defining destinations in multiple firewall ACL rules.`}</p>
                    </div>
                  )}
                  {isPresent(aliasesOptions) && aliasesOptions.length > 0 && (
                    <>
                      <DescriptionBlock title="Aliases">
                        <p>{`Aliases can optionally define some or all of the manual destination settings. They are combined with the values you specify to form the final destination for firewall rule generation.`}</p>
                      </DescriptionBlock>
                      <SizedBox height={ThemeSpacing.Lg} />
                      <form.AppField name="aliases">
                        {(field) => (
                          <>
                            <ButtonsGroup>
                              <Button
                                variant="outlined"
                                text="Apply aliases"
                                disabled={aliasesOptions?.length === 0}
                                onClick={() => {
                                  useSelectionModal.setState({
                                    isOpen: true,
                                    onSubmit: (selected) => {
                                      field.handleChange(new Set(selected as number[]));
                                    },
                                    options: aliasesOptions,
                                    selected: new Set(field.state.value),
                                    title: 'Select Aliases',
                                  });
                                }}
                              />
                            </ButtonsGroup>
                            <SizedBox height={ThemeSpacing.Xl} />
                            {isPresent(aliasesOptions) &&
                              aliasesOptions
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
                          </>
                        )}
                      </form.AppField>
                    </>
                  )}
                  <Divider spacing={ThemeSpacing.Xl} />
                  <DescriptionBlock title="Addresses/Ranges">
                    <p>
                      {`Define the IP addresses or ranges that form the destination of this ACL rule.`}
                    </p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="any_address">
                    {(field) => <field.FormToggle label="Any IP Address" />}
                  </form.AppField>
                  <form.Subscribe selector={(s) => !s.values.any_address}>
                    {(open) => (
                      <Fold open={open}>
                        <SizedBox height={ThemeSpacing.Xl} />
                        <form.AppField name="addresses">
                          {(field) => (
                            <field.FormTextarea label="IPv4/IPv6 CIDR ranges or addresses (or multiple values separated by commas)" />
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
                  <DescriptionBlock title="Ports">
                    <p>
                      {`You may specify the exact ports accessible to users in this location.`}
                    </p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="any_port">
                    {(field) => <field.FormToggle label="Any port" />}
                  </form.AppField>
                  <form.Subscribe selector={(s) => !s.values.any_port}>
                    {(open) => (
                      <Fold open={open}>
                        <SizedBox height={ThemeSpacing.Xl} />
                        <form.AppField name="ports">
                          {(field) => (
                            <field.FormInput label="Manually defined ports (or multiple values separated by commas)" />
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
                  <DescriptionBlock title="Protocols">
                    <p>
                      {`By default, all protocols are allowed for this location. You can change this configuration, but at least one protocol must remain selected.`}
                    </p>
                  </DescriptionBlock>
                  <SizedBox height={ThemeSpacing.Xl} />
                  <form.AppField name="any_protocol">
                    {(field) => <field.FormToggle label="Any protocol" />}
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
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="enrollment">
          <AppText font={TextStyle.TBodyPrimary600}>{`Permissions`}</AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title="Permitted Users & Devices">
            <p>{`Define who should be granted access. Only the entities you list here will be allowed through.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          {isPresent(usersOptions) && (
            <form.Subscribe selector={(s) => s.values.allow_all_users}>
              {(allowAllValue) => (
                <form.AppField name="allowed_users">
                  {(field) => (
                    <field.FormSelectMultiple
                      toggleValue={allowAllValue}
                      toggleText="All users have access"
                      counterText={(counter) => `Users ${counter}`}
                      editText={`Edit users`}
                      modalTitle="Select allowed users"
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
                      counterText={(counter) => `Groups ${counter}`}
                      editText="Edit groups"
                      modalTitle="Select allowed groups"
                      toggleText="All groups have access"
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
                      counterText={(counter) => `Devices ${counter}`}
                      editText="Edit devices"
                      modalTitle="Select allowed devices"
                      toggleText="All network devices have access"
                    />
                  )}
                </form.AppField>
              )}
            </form.Subscribe>
          )}
        </MarkedSection>
        {/* <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="lock-closed">
          <AppText font={TextStyle.TBodyPrimary600}>{`Restrictions`}</AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title="Restrict access">
            <p>{`If needed, you may exclude specific users, groups, or devices from accessing this location.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <Checkbox
            active={restrictionsPresent}
            onClick={() => {
              setRestrictionsPresent((s) => !s);
            }}
            text="Add restriction settings"
          />
          <Fold open={restrictionsPresent}>
            <SizedBox height={ThemeSpacing.Xl2} />
            {isPresent(usersOptions) && (
              <form.AppField name="denied_users">
                {(field) => (
                  <field.FormSelectMultiple
                    toggleText="Exclude specific users"
                    counterText={(counter) => `Users ${counter}`}
                    editText={`Edit users`}
                    modalTitle="Select restricted users"
                    options={usersOptions}
                  />
                )}
              </form.AppField>
            )}
            <Divider spacing={ThemeSpacing.Lg} />
            {isPresent(groupsOptions) && (
              <form.AppField name="denied_groups">
                {(field) => (
                  <field.FormSelectMultiple
                    options={groupsOptions}
                    counterText={(counter) => `Groups ${counter}`}
                    editText="Edit groups"
                    modalTitle="Select restricted groups"
                    toggleText="Exclude specific groups"
                  />
                )}
              </form.AppField>
            )}
            <Divider spacing={ThemeSpacing.Lg} />
            {isPresent(networkDevicesOptions) && (
              <form.AppField name="denied_devices">
                {(field) => (
                  <field.FormSelectMultiple
                    options={networkDevicesOptions}
                    counterText={(counter) => `Devices ${counter}`}
                    editText="Edit devices"
                    modalTitle="Select restricted devices"
                    toggleText="Exclude specific network devices"
                  />
                )}
              </form.AppField>
            )}
          </Fold>
        </MarkedSection> */}
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

type AliasDataBlockProps = {
  values: string[];
};

const AliasDataBlock = ({ values }: AliasDataBlockProps) => {
  if (values.length === 0) return null;
  return (
    <div className="alias-data-block">
      <div className="top">
        <p>{`Data from aliases`}</p>
      </div>
      <div className="content-track">
        {values.map((value) => (
          <Chip key={value} text={value} />
        ))}
        {values.length > 4 && (
          <button
            onClick={() => {
              openModal(ModalName.DisplayList, {
                title: 'Data from aliases',
                data: values,
              });
            }}
          >
            <span>{`Show all`}</span>
          </button>
        )}
      </div>
    </div>
  );
};
