import { intersection } from 'lodash-es';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import { Controls } from '../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../shared/components/DescriptionBlock/DescriptionBlock';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { AppText } from '../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../shared/defguard-ui/components/Toggle/Toggle';
import { TextStyle, ThemeSpacing, ThemeVariable } from '../../shared/defguard-ui/types';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { aclDestinationValidator, aclPortsValidator } from '../../shared/validators';

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
          protocols: z.number().array(),
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
      protocols: [],
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
          <Checkbox disabled active={false} text="Add manual destination settings" />
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="enrollment">
          <AppText font={TextStyle.TBodyPrimary600}>{`Permissions`}</AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title="Permitted Users & Devices">
            <p>{`Define who should be granted access. Only the entities you list here will be allowed through.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <Toggle disabled active label="All users have access" />
          <Divider spacing={ThemeSpacing.Lg} />
          <Toggle disabled active label="All groups have access" />
          <Divider spacing={ThemeSpacing.Lg} />
          <Toggle disabled active label="All network devices have access" />
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="lock-closed">
          <AppText font={TextStyle.TBodyPrimary600}>{`Restrictions`}</AppText>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title="Restrict access">
            <p>{`If needed, you may exclude specific users, groups, or devices from accessing this location.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Xl} />
          <Checkbox active={false} disabled text="Add restriction settings" />
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <Controls>
          <Toggle label="Enable rule" active disabled />
          <div className="right">
            <Button text="Create rule" disabled />
          </div>
        </Controls>
      </form.AppForm>
    </form>
  );
};
