import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { Link, useNavigate, useParams } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import type { Edge } from '../../shared/api/types';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { EditPageControls } from '../../shared/components/EditPageControls/EditPageControls';
import { EditPageFormSection } from '../../shared/components/EditPageFormSection/EditPageFormSection';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { useAppForm } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';
import { getEdgeQueryOptions } from '../../shared/query';

const breadcrumbsLinks = [
  <Link key={0} to="/edge">
    Edge components
  </Link>,
  <Link key={1} to="/edge">
    Edit
  </Link>,
];

export const EditEdgePage = () => {
  const { edgeId: paramsId } = useParams({
    from: '/_authorized/_default/edge/$edgeId/edit',
  });
  const { data: edge } = useSuspenseQuery(getEdgeQueryOptions(Number(paramsId)));
  return (
    <EditPage
      pageTitle="Edge component"
      links={breadcrumbsLinks}
      headerProps={{ title: 'Edit Edge component' }}
    >
      <EditEdgeForm edge={edge} />
    </EditPage>
  );
};

const formSchema = z.object({
  name: z.string(m.form_error_required()).min(1, m.form_error_required()),
  address: z.string(),
  port: z.number(),
  public_address: z.string(),
});

type FormFields = z.infer<typeof formSchema>;

const EditEdgeForm = ({ edge }: { edge: Edge }) => {
  const navigate = useNavigate();

  const { mutateAsync: editEdge } = useMutation({
    mutationFn: api.edge.editEdge,
    meta: {
      invalidate: ['edge'],
    },
    onSuccess: () => {
      navigate({
        to: '/edge',
        replace: true,
      });
    },
  });

  const { mutate: deleteEdge, isPending: deletePending } = useMutation({
    mutationFn: () => api.edge.deleteEdge(edge.id),
    meta: {
      invalidate: ['edge'],
    },
    onSuccess: () => {
      navigate({
        // TODO(jck)
        to: '/locations',
        replace: true,
      });
    },
  });

  const defaultValues = useMemo((): FormFields => ({ ...edge }), [edge]);

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await editEdge({
        ...value,
        id: edge.id,
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
        <EditPageFormSection label="General information">
          <form.AppField name="name">
            {(field) => <field.FormInput required label="Name" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="address">
            {(field) => <field.FormInput required disabled label="IP or Domain" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="port">
            {(field) => <field.FormInput required disabled label="gRPC port" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="public_address">
            {(field) => <field.FormInput required disabled label="Public domain" />}
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
                text: 'Delete',
                onClick: () => {
                  deleteEdge();
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
