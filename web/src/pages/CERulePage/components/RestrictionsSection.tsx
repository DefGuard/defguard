import type { Dispatch, SetStateAction } from 'react';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Checkbox } from '../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TextStyle, ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import type { CERuleFormApi, FormFieldRenderer } from '../types';

type Props = {
  form: unknown;
  usersOptions: SelectionOption<number>[] | undefined;
  groupsOptions: SelectionOption<number>[] | undefined;
  networkDevicesOptions: SelectionOption<number>[] | undefined;
  restrictUsers: boolean;
  restrictGroups: boolean;
  restrictDevices: boolean;
  setRestrictUsers: Dispatch<SetStateAction<boolean>>;
  setRestrictGroups: Dispatch<SetStateAction<boolean>>;
  setRestrictDevices: Dispatch<SetStateAction<boolean>>;
};

export const RestrictionsSection = ({
  form,
  usersOptions,
  groupsOptions,
  networkDevicesOptions,
  restrictUsers,
  restrictGroups,
  restrictDevices,
  setRestrictUsers,
  setRestrictGroups,
  setRestrictDevices,
}: Props) => {
  const Form = form as CERuleFormApi;

  return (
    <MarkedSection icon="lock-closed">
      <AppText font={TextStyle.TBodyPrimary600}>{`Restrictions`}</AppText>
      <SizedBox height={ThemeSpacing.Xl} />
      <DescriptionBlock title="Limit access">
        <p>{`Choose who or what should be blocked from accessing this location.`}</p>
      </DescriptionBlock>
      <SizedBox height={ThemeSpacing.Xl} />
      {isPresent(usersOptions) && (
        <div className="restriction-block">
          <div className="restriction-toggle">
            <Checkbox
              active={restrictUsers}
              onClick={() => {
                setRestrictUsers((current) => !current);
              }}
              text="Limit access for users"
            />
          </div>
          <Fold open={restrictUsers}>
            <div className="restriction-body">
              <div className="restriction-radio">
                <Form.AppField name="deny_all_users">
                  {(field) => {
                    const Field = field as FormFieldRenderer;
                    return <Field.FormRadio text="Exclude all users" value={true} />;
                  }}
                </Form.AppField>
                <Form.AppField name="deny_all_users">
                  {(field) => {
                    const Field = field as FormFieldRenderer;
                    return (
                      <Field.FormRadio text="Exclude specific users" value={false} />
                    );
                  }}
                </Form.AppField>
              </div>
              <Form.Subscribe
                selector={(s) => {
                  const state = s as { values: { deny_all_users: boolean } };
                  return state.values.deny_all_users === false && restrictUsers;
                }}
              >
                {(open) => (
                  <Fold open={Boolean(open)}>
                    {isPresent(usersOptions) && (
                      <Form.AppField name="denied_users">
                        {(field) => {
                          const Field = field as FormFieldRenderer;
                          return (
                            <Field.FormSelectMultiple
                              toggleValue={!open}
                              onToggleChange={() => {}}
                              counterText={(counter: number) => `Users ${counter}`}
                              editText="Edit users"
                              modalTitle="Select restricted users"
                              options={usersOptions}
                            />
                          );
                        }}
                      </Form.AppField>
                    )}
                  </Fold>
                )}
              </Form.Subscribe>
            </div>
          </Fold>
        </div>
      )}
      <Divider spacing={ThemeSpacing.Lg} />
      {isPresent(groupsOptions) && (
        <div className="restriction-block">
          <div className="restriction-toggle">
            <Checkbox
              active={restrictGroups}
              onClick={() => {
                setRestrictGroups((current) => !current);
              }}
              text="Limit access for groups"
            />
          </div>
          <Fold open={restrictGroups}>
            <div className="restriction-body">
              <div className="restriction-radio">
                <Form.AppField name="deny_all_groups">
                  {(field) => {
                    const Field = field as FormFieldRenderer;
                    return <Field.FormRadio text="Exclude all groups" value={true} />;
                  }}
                </Form.AppField>
                <Form.AppField name="deny_all_groups">
                  {(field) => {
                    const Field = field as FormFieldRenderer;
                    return (
                      <Field.FormRadio text="Exclude specific groups" value={false} />
                    );
                  }}
                </Form.AppField>
              </div>
              <Form.Subscribe
                selector={(s) =>
                  (s as { values: { deny_all_groups: boolean } }).values
                    .deny_all_groups === false && restrictGroups
                }
              >
                {(open) => (
                  <Fold open={Boolean(open)}>
                    {isPresent(groupsOptions) && (
                      <Form.AppField name="denied_groups">
                        {(field) => {
                          const Field = field as FormFieldRenderer;
                          return (
                            <Field.FormSelectMultiple
                              toggleValue={!open}
                              onToggleChange={() => {}}
                              counterText={(counter: number) => `Groups ${counter}`}
                              editText="Edit groups"
                              modalTitle="Select restricted groups"
                              options={groupsOptions}
                            />
                          );
                        }}
                      </Form.AppField>
                    )}
                  </Fold>
                )}
              </Form.Subscribe>
            </div>
          </Fold>
          <Divider spacing={ThemeSpacing.Lg} />
        </div>
      )}
      {isPresent(networkDevicesOptions) && (
        <div className="restriction-block">
          <div className="restriction-toggle">
            <Checkbox
              active={restrictDevices}
              onClick={() => {
                setRestrictDevices((current) => !current);
              }}
              text="Limit access for devices"
            />
          </div>
          <Fold open={restrictDevices}>
            <div className="restriction-body">
              <div className="restriction-radio">
                <Form.AppField name="deny_all_network_devices">
                  {(field) => {
                    const Field = field as FormFieldRenderer;
                    return <Field.FormRadio text="Exclude all devices" value={true} />;
                  }}
                </Form.AppField>
                <Form.AppField name="deny_all_network_devices">
                  {(field) => {
                    const Field = field as FormFieldRenderer;
                    return (
                      <Field.FormRadio text="Exclude specific devices" value={false} />
                    );
                  }}
                </Form.AppField>
              </div>
              <Form.Subscribe
                selector={(s) =>
                  (s as { values: { deny_all_network_devices: boolean } }).values
                    .deny_all_network_devices === false && restrictDevices
                }
              >
                {(open) => (
                  <Fold open={Boolean(open)}>
                    {isPresent(networkDevicesOptions) && (
                      <Form.AppField name="denied_network_devices">
                        {(field) => {
                          const Field = field as FormFieldRenderer;
                          return (
                            <Field.FormSelectMultiple
                              toggleValue={!open}
                              onToggleChange={() => {}}
                              counterText={(counter: number) => `Devices ${counter}`}
                              editText="Edit devices"
                              modalTitle="Select restricted devices"
                              options={networkDevicesOptions}
                            />
                          );
                        }}
                      </Form.AppField>
                    )}
                  </Fold>
                )}
              </Form.Subscribe>
            </div>
          </Fold>
        </div>
      )}
    </MarkedSection>
  );
};
