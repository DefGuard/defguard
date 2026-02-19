import { useMutation, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { SettingsGatewayNotifications } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Fold } from '../../../shared/defguard-ui/components/Fold/Fold';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { useApp } from '../../../shared/hooks/useApp';
import { getSettingsQueryOptions } from '../../../shared/query';

const breadcrumbsLinks = [
  <Link to="/settings" search={{ tab: 'notifications' }} key={0}>
    Notifications
  </Link>,
  <Link key={1} to="/settings/gateway-notifications">
    Gateway notifications
  </Link>,
];

export const SettingsGatewayNotificationsPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);

  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbsLinks} />
      <SettingsLayout>
        <SettingsHeader
          icon="notification"
          title="Gateway notifications"
          subtitle="Here you can manage email notifications."
        />
        {isPresent(settings) && (
          <SettingsCard>
            <Content settings={settings} />
          </SettingsCard>
        )}
      </SettingsLayout>
    </Page>
  );
};

const formSchema = z.object({
  gateway_disconnect_notifications_enabled: z.boolean(),
  gateway_disconnect_notifications_inactivity_threshold: z
    .number(m.form_error_required())
    .min(0, m.form_min_value({ value: 0 })),
  gateway_disconnect_notifications_reconnect_notification_enabled: z.boolean(),
});

type FormFields = z.infer<typeof formSchema>;

const Content = ({ settings }: { settings: SettingsGatewayNotifications }) => {
  const smtp = useApp((s) => s.appInfo.smtp_enabled);
  const formDisabled = !smtp;
  const defaultValues = useMemo(
    (): FormFields => ({
      gateway_disconnect_notifications_enabled:
        settings.gateway_disconnect_notifications_enabled ?? false,
      gateway_disconnect_notifications_inactivity_threshold:
        settings.gateway_disconnect_notifications_inactivity_threshold ?? 5,
      gateway_disconnect_notifications_reconnect_notification_enabled:
        settings.gateway_disconnect_notifications_reconnect_notification_enabled ?? false,
    }),
    [settings],
  );

  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: ['settings'],
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      mutateAsync(value);
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
        {!smtp && (
          <>
            <InfoBanner
              icon="info-outlined"
              variant="warning"
              text={'To enable notifications you must first configure SMTP.'}
            />
            <SizedBox height={ThemeSpacing.Xl} />
          </>
        )}
        <form.AppField name="gateway_disconnect_notifications_enabled">
          {(field) => (
            <field.FormInteractiveBlock
              variant="toggle"
              title="Gateway disconnect notifications"
              content="Send email notification to admin users once a gateway is disconnected"
              disabled={formDisabled}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="gateway_disconnect_notifications_reconnect_notification_enabled">
          {(field) => (
            <field.FormInteractiveBlock
              variant="toggle"
              title="Gateway reconnect notifications"
              content="Send email notification to admin users once a gateway is reconnected"
              disabled={formDisabled}
            >
              <form.Subscribe
                selector={(s) =>
                  s.values.gateway_disconnect_notifications_reconnect_notification_enabled
                }
              >
                {(enabled) => (
                  <Fold open={enabled && !formDisabled}>
                    <SizedBox height={ThemeSpacing.Lg} />
                    <form.AppField name="gateway_disconnect_notifications_inactivity_threshold">
                      {(field) => (
                        <field.FormInput
                          required
                          label="Gateway inactive time (minutes)"
                          type="number"
                          disabled={formDisabled}
                        />
                      )}
                    </form.AppField>
                  </Fold>
                )}
              </form.Subscribe>
            </field.FormInteractiveBlock>
          )}
        </form.AppField>
        <form.Subscribe
          selector={(s) => ({
            isDefault: s.isDefaultValue || s.isPristine,
            isSubmitting: s.isSubmitting,
          })}
        >
          {({ isDefault, isSubmitting }) => (
            <Controls>
              <div className="right">
                <Button
                  variant="primary"
                  text={m.controls_save_changes()}
                  disabled={isDefault}
                  loading={isSubmitting}
                  onClick={() => {
                    form.handleSubmit();
                  }}
                />
              </div>
            </Controls>
          )}
        </form.Subscribe>
      </form.AppForm>
    </form>
  );
};
