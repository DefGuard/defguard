import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { intersection } from 'lodash-es';
import { useMemo, useState } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import { AclProtocol, AclProtocolName } from '../../shared/api/types';
import { Card } from '../../shared/components/Card/Card';
import { Controls } from '../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../shared/components/DescriptionBlock/DescriptionBlock';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { useSelectionModal } from '../../shared/components/modals/SelectionModal/useSelectionModal';
import type { SelectionSectionOption } from '../../shared/components/SelectionSection/type';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { ButtonsGroup } from '../../shared/defguard-ui/components/ButtonsGroup/ButtonsGroup';
import { Checkbox } from '../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Chip } from '../../shared/defguard-ui/components/Chip/Chip';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../shared/defguard-ui/components/Fold/Fold';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../shared/defguard-ui/components/Toggle/Toggle';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import { isPresent } from '../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import {
  getAliasesQueryOptions,
  getGroupsInfoQueryOptions,
  getNetworkDevicesQueryOptions,
  getUsersQueryOptions,
} from '../../shared/query';
import { aclDestinationValidator, aclPortsValidator } from '../../shared/validators';
import aliasesEmptyImage from './assets/aliases-empty-icon.png';

const availableProtocols = Object.keys(AclProtocol) as Array<keyof typeof AclProtocol>;

export const CERulePage = () => {
  return (
    <EditPage
      id="ce-rule-page"
      pageTitle="Rules"
      headerProps={{
        icon: 'add-rule',
        title: `Create rule for firewall`,
        subtitle: `Define who can access specific locations and which IPs, ports, and protocols are allowed. Use firewall rules to grant or restrict access for users and groups, ensuring your network stays secure and controlled.`,
      }}
    >
      <Content />
    </EditPage>
  );
};

const Content = () => {
  const [destinationAllAddresses, setDestinationAllAddresses] = useState<boolean>(true);
  const [destinationAllPorts, setDestinationAllPorts] = useState<boolean>(true);
  const [destinationAllProtocols, setDestinationAllProtocols] = useState<boolean>(true);

  const { data: users } = useQuery(getUsersQueryOptions);

  const usersOptions = useMemo(() => {
    if (isPresent(users)) {
      return users.map(
        (user): SelectionSectionOption<number> => ({
          id: user.id,
          label: user.username,
          meta: user,
          searchFields: [user.username, user.email, user.first_name, user.last_name],
        }),
        [],
      );
    }
  }, [users]);

  const { data: aliases } = useQuery(getAliasesQueryOptions);

  const aliasesOptions = useMemo(() => {
    if (isPresent(aliases)) {
      return aliases.map(
        (alias): SelectionSectionOption<number> => ({
          id: alias.id,
          label: alias.name,
          meta: alias,
        }),
        [],
      );
    }
  }, [aliases]);

  const { data: groups } = useQuery(getGroupsInfoQueryOptions);
  const groupsOptions = useMemo(() => {
    if (isPresent(groups)) {
      return groups.map(
        (group): SelectionSectionOption<string> => ({
          id: group.name,
          label: group.name,
          meta: group,
        }),
      );
    }
  }, [groups]);

  const { data: networkDevices } = useQuery(getNetworkDevicesQueryOptions);
  const networkDevicesOptions = useMemo(() => {
    if (isPresent(networkDevices)) {
      return networkDevices.map(
        (device): SelectionSectionOption<number> => ({
          id: device.id,
          label: device.name,
          meta: device,
        }),
      );
    }
  }, [networkDevices]);

  const [restrictionsPresent, setRestrictionsPresent] = useState(false);
  const [manualDestination, setManualDestination] = useState(false);

  const formSchema = useMemo(
    () =>
      z
        .object({
          name: z.string(m.form_error_required()).min(1, m.form_error_required()),
          networks: z.number().array(),
          expires: z.string().nullable(),
          enabled: z.boolean(),
          all_networks: z.boolean(),
          allow_all_users: z.boolean(),
          deny_all_users: z.boolean(),
          allow_all_network_devices: z.boolean(),
          deny_all_network_devices: z.boolean(),
          allowed_users: z.number().array(),
          denied_users: z.number().array(),
          allowed_groups: z.number().array(),
          denied_groups: z.number().array(),
          allowed_devices: z.number().array(),
          denied_devices: z.number().array(),
          aliases: z.number().array(),
          protocols: z.set(z.number()),
          destination: aclDestinationValidator,
          ports: aclPortsValidator,
        })
        .superRefine((vals, ctx) => {
          // check for collisions
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
            if (intersection(vals.allowed_devices, vals.denied_devices).length) {
              ctx.addIssue({
                path: ['allowed_devices'],
                code: 'custom',
                message,
              });
              ctx.addIssue({
                path: ['denied_devices'],
                code: 'custom',
                message,
              });
            }
          }

          // check if one of allowed users/groups/devices fields is set
          const isAllowConfigured =
            vals.allow_all_users ||
            vals.allow_all_network_devices ||
            vals.allowed_users.length !== 0 ||
            vals.allowed_groups.length !== 0 ||
            vals.allowed_devices.length !== 0;
          if (!isAllowConfigured) {
            const message = 'Must configure some allowed users, groups or devices';

            ctx.addIssue({
              path: ['allow_all_users'],
              code: 'custom',
              message,
            });
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
              path: ['allow_all_network_devices'],
              code: 'custom',
              message,
            });
            ctx.addIssue({
              path: ['allowed_devices'],
              code: 'custom',
              message,
            });
          }
        }),
    [],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      name: '',
      destination: '',
      ports: '',
      aliases: [],
      allowed_devices: [],
      allowed_groups: [],
      allowed_users: [],
      denied_devices: [],
      denied_groups: [],
      denied_users: [],
      networks: [],
      protocols: new Set(),
      all_networks: false,
      allow_all_users: false,
      allow_all_network_devices: false,
      deny_all_users: false,
      deny_all_network_devices: false,
      enabled: true,
      expires: null,
    }),
    [],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
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
          <Toggle active={false} disabled label="Include all locations" />
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="location-tracking">
          <AppText font={TextStyle.TBodyPrimary600}>{`Destination`}</AppText>
          <SizedBox height={ThemeSpacing.Sm} />
          <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
            {`You can add additional destinations to this rule to extend its scope. These destinations are configured separately in the 'Destinations' section.`}
          </AppText>
          <Divider text="or/and" spacing={ThemeSpacing.Lg} />
          <DescriptionBlock title={`Define destination manually`}>
            <p>{`Manually configure destinations parameters for this rule.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <Checkbox
            text="Add manual destination settings"
            active={manualDestination}
            onClick={() => {
              setManualDestination((s) => !s);
            }}
          />
          <Fold open={manualDestination}>
            <SizedBox height={ThemeSpacing.Xl2} />
            <Card>
              {isPresent(aliasesOptions) && aliasesOptions.length === 0 && (
                <div className="no-aliases-block">
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
                                  field.handleChange(selected as number[]);
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
                            .filter((alias) => field.state.value.includes(alias.id))
                            .map((option) => (
                              <Chip size="sm" text={option.label} key={option.id} />
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
              <Toggle
                active={destinationAllAddresses}
                onClick={() => {
                  setDestinationAllAddresses((s) => !s);
                }}
                label="Any IP Address"
              />
              <Fold open={!destinationAllAddresses}>
                <SizedBox height={ThemeSpacing.Xl} />
                <form.AppField name="destination">
                  {(field) => (
                    <field.FormTextarea label="IPv4/IPv6 CIDR ranges or addresses (or multiple values separated by commas)" />
                  )}
                </form.AppField>
              </Fold>
              <Divider spacing={ThemeSpacing.Xl} />
              <DescriptionBlock title="Ports">
                <p>
                  {`You may specify the exact ports accessible to users in this location.`}
                </p>
              </DescriptionBlock>
              <SizedBox height={ThemeSpacing.Xl} />
              <Toggle
                label="All ports"
                active={destinationAllPorts}
                onClick={() => {
                  setDestinationAllPorts((s) => !s);
                }}
              />
              <Fold open={!destinationAllPorts}>
                <SizedBox height={ThemeSpacing.Xl} />
                <form.AppField name="ports">
                  {(field) => (
                    <field.FormInput label="Manually defined ports (or multiple values separated by commas)" />
                  )}
                </form.AppField>
              </Fold>
              <Divider spacing={ThemeSpacing.Xl} />
              <DescriptionBlock title="Protocols">
                <p>
                  {`By default, all protocols are allowed for this location. You can change this configuration, but at least one protocol must remain selected.`}
                </p>
              </DescriptionBlock>
              <SizedBox height={ThemeSpacing.Xl} />
              <Toggle
                label="All protocols"
                active={destinationAllProtocols}
                onClick={() => {
                  setDestinationAllProtocols((s) => !s);
                }}
              />
              <Fold open={!destinationAllProtocols}>
                <SizedBox height={ThemeSpacing.Xl2} />
                <div className="protocols-selection">
                  {availableProtocols.map((protocolKey) => {
                    const value = AclProtocol[protocolKey];
                    const name = AclProtocolName[value];
                    return (
                      <form.AppField name="protocols" key={protocolKey}>
                        {(field) => <field.FormCheckbox value={value} text={name} />}
                      </form.AppField>
                    );
                  })}
                </div>
              </Fold>
            </Card>
          </Fold>
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
            <form.AppField name="allowed_users">
              {(field) => (
                <field.FormSelectMultiple
                  toggleText="All users have access"
                  counterText={(counter) => `Users ${counter}`}
                  editText={`Edit users`}
                  modalTitle="Select allowed users"
                  options={usersOptions}
                />
              )}
            </form.AppField>
          )}
          <Divider spacing={ThemeSpacing.Lg} />
          {isPresent(groupsOptions) && (
            <form.AppField name="allowed_groups">
              {(field) => (
                <field.FormSelectMultiple
                  options={groupsOptions}
                  counterText={(counter) => `Groups ${counter}`}
                  editText="Edit groups"
                  modalTitle="Select allowed groups"
                  toggleText="All groups have access"
                />
              )}
            </form.AppField>
          )}
          <Divider spacing={ThemeSpacing.Lg} />
          {isPresent(networkDevicesOptions) && (
            <form.AppField name="allowed_devices">
              {(field) => (
                <field.FormSelectMultiple
                  options={networkDevicesOptions}
                  counterText={(counter) => `Devices ${counter}`}
                  editText="Edit devices"
                  modalTitle="Select allowed devices"
                  toggleText="All network devices have access"
                />
              )}
            </form.AppField>
          )}
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
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
              <form.AppField name="denied_groups">
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
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <Controls>
          <form.AppField name="enabled">
            {(field) => <field.FormToggle label="Enable rule" />}
          </form.AppField>
          <div className="right">
            <Button text="Create rule" disabled />
          </div>
        </Controls>
      </form.AppForm>
    </form>
  );
};
