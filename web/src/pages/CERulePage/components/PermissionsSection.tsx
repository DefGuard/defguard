import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import type { SelectionOption } from '../../../shared/components/SelectionSection/type';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
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
};

export const PermissionsSection = ({
  form,
  usersOptions,
  groupsOptions,
  networkDevicesOptions,
}: Props) => {
  const Form = form as CERuleFormApi;

  return (
    <MarkedSection icon="enrollment">
      <AppText font={TextStyle.TBodyPrimary600}>{`Permissions`}</AppText>
      <SizedBox height={ThemeSpacing.Xl} />
      <DescriptionBlock title="Permitted Users & Devices">
        <p>{`Define who should be granted access. Only the entities you list here will be allowed through.`}</p>
      </DescriptionBlock>
      <SizedBox height={ThemeSpacing.Xl} />
      {isPresent(usersOptions) && (
        <Form.Subscribe
          selector={(s) =>
            (s as { values: { allow_all_users: boolean } }).values.allow_all_users
          }
        >
          {(allowAllValue) => (
            <Form.AppField name="allowed_users">
              {(field) => {
                const Field = field as FormFieldRenderer;
                return (
                  <Field.FormSelectMultiple
                    toggleValue={allowAllValue}
                    toggleText="All users have access"
                    counterText={(counter: number) => `Users ${counter}`}
                    editText={`Edit users`}
                    modalTitle="Select allowed users"
                    options={usersOptions}
                    onToggleChange={(value: boolean) => {
                      Form.setFieldValue('allow_all_users', value);
                    }}
                  />
                );
              }}
            </Form.AppField>
          )}
        </Form.Subscribe>
      )}
      <Divider spacing={ThemeSpacing.Lg} />
      {isPresent(groupsOptions) && (
        <Form.Subscribe
          selector={(s) =>
            (s as { values: { allow_all_groups: boolean } }).values.allow_all_groups
          }
        >
          {(allAllowedValue) => (
            <Form.AppField name="allowed_groups">
              {(field) => {
                const Field = field as FormFieldRenderer;
                return (
                  <Field.FormSelectMultiple
                    toggleValue={allAllowedValue}
                    onToggleChange={(value: boolean) => {
                      Form.setFieldValue('allow_all_groups', value);
                    }}
                    options={groupsOptions}
                    counterText={(counter: number) => `Groups ${counter}`}
                    editText="Edit groups"
                    modalTitle="Select allowed groups"
                    toggleText="All groups have access"
                  />
                );
              }}
            </Form.AppField>
          )}
        </Form.Subscribe>
      )}
      <Divider spacing={ThemeSpacing.Lg} />
      {isPresent(networkDevicesOptions) && (
        <Form.Subscribe
          selector={(s) =>
            (s as { values: { allow_all_network_devices: boolean } }).values
              .allow_all_network_devices
          }
        >
          {(allowAllValue) => (
            <Form.AppField name="allowed_network_devices">
              {(field) => {
                const Field = field as FormFieldRenderer;
                return (
                  <Field.FormSelectMultiple
                    toggleValue={allowAllValue}
                    onToggleChange={(value: boolean) => {
                      Form.setFieldValue('allow_all_network_devices', value);
                    }}
                    options={networkDevicesOptions}
                    counterText={(counter: number) => `Devices ${counter}`}
                    editText="Edit devices"
                    modalTitle="Select allowed devices"
                    toggleText="All network devices have access"
                  />
                );
              }}
            </Form.AppField>
          )}
        </Form.Subscribe>
      )}
    </MarkedSection>
  );
};
