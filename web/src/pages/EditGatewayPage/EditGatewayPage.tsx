import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { Link, useNavigate, useParams } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { Gateway } from '../../shared/api/types';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { EditPageControls } from '../../shared/components/EditPageControls/EditPageControls';
import { EditPageFormSection } from '../../shared/components/EditPageFormSection/EditPageFormSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { getGatewayQueryOptions } from '../../shared/query';

export const EditGatewayPage = () => {
  const { gatewayId } = useParams({
    from: '/_authorized/_default/gateway/$gatewayId/edit',
  });
  const { data: gateway } = useSuspenseQuery(getGatewayQueryOptions(Number(gatewayId)));
  const breadcrumbsLinks = [
    <Link key={0} to="/locations">
      {m.gateway_title()}
    </Link>,
    <Link key={1} to="/gateway/$gatewayId/edit" params={{ gatewayId }}>
      {gateway.name}
    </Link>,
  ];
  return (
    <EditPage
      pageTitle={m.gateway_title()}
      links={breadcrumbsLinks}
      headerProps={{ title: m.gateway_edit_title() }}
    >
      <EditGatewayForm gateway={gateway} />
    </EditPage>
  );
};

const formSchema = z.object({
  name: z.string(m.form_error_required()).min(1, m.form_error_required()),
  address: z.string().nullable(),
  port: z.number().nullable(),
  connected_at: z.string().nullable(),
  disconnected_at: z.string().nullable(),
  modified_at: z.string(),
  modified_by: z.number(),
  version: z.string().nullable(),
  location_id: z.number(),
});

type FormFields = z.infer<typeof formSchema>;

const EditGatewayForm = ({ gateway }: { gateway: Gateway }) => {
  const navigate = useNavigate();

  const { mutateAsync: editGateway } = useMutation({
    mutationFn: api.gateway.editGateway,
    meta: {
      invalidate: ['gateway'],
    },
    onSuccess: () => {
      Snackbar.success(m.gateway_edit_success());
    },
    onError: () => {
      Snackbar.error(m.gateway_edit_failed());
    },
  });

  const { mutate: deleteGateway, isPending: deletePending } = useMutation({
    mutationFn: () => api.gateway.deleteGateway(gateway.id),
    meta: {
      invalidate: ['gateway'],
    },
    onSuccess: () => {
      navigate({
        to: '/locations',
        replace: true,
      });
      Snackbar.success(m.gateway_delete_success());
    },
    onError: () => {
      Snackbar.error(m.gateway_delete_failed());
    },
  });

  const defaultValues = useMemo((): FormFields => ({ ...gateway }), [gateway]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await editGateway({
        ...value,
        id: gateway.id,
      });
      form.reset(value);
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
        <EditPageFormSection label={m.gateway_edit_general_info()}>
          <form.AppField name="name">
            {(field) => <field.FormInput required label={m.gateway_edit_name()} />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="address">
            {(field) => <field.FormInput disabled label={m.gateway_edit_address()} />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="port">
            {(field) => <field.FormInput disabled label={m.gateway_edit_port()} />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
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
                text: m.gateway_edit_delete(),
                onClick: () => {
                  deleteGateway();
                },
                loading: deletePending,
                disabled: isSubmitting,
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
