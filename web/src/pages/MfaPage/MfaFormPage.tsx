import { useMutation } from '@tanstack/react-query';
import { Link, useNavigate } from '@tanstack/react-router';
import type { AxiosError } from 'axios';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../paraglide/messages';
import api from '../../shared/api/api';
import {
  type CreateMfaFlowRequest,
  type MfaFlowDetailResponse,
  type MfaFlowErrorResponse,
  MfaFlowMethod,
  type UpdateMfaFlowRequest,
} from '../../shared/api/types';
import { Controls } from '../../shared/components/Controls/Controls';
import { EditPage } from '../../shared/components/EditPage/EditPage';
import { MfaConfiguration } from '../../shared/components/MfaConfiguration/MfaConfiguration';
import type { MfaConfigurationStepData } from '../../shared/components/MfaConfiguration/types';
import { Button } from '../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { useFormFieldError } from '../../shared/defguard-ui/hooks/useFormFieldError';
import { Snackbar } from '../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { useAppForm, useFieldContext } from '../../shared/form';
import { formChangeLogic } from '../../shared/formLogic';

const formSchema = z.object({
  title: z.string(m.form_error_required()).trim().min(1, m.form_error_required()),
  steps: z
    .array(
      z.object({
        id: z.union([z.string(), z.number()]),
        methods: z.array(z.enum(MfaFlowMethod)).min(1, m.mfa_flow_method_required()),
      }),
    )
    .min(1, m.mfa_flow_step_required()),
});

/** Maps a structured MFA flow API failure to user-facing copy. */
const getSaveErrorMessage = (error: AxiosError<MfaFlowErrorResponse>): string => {
  const code = error.response?.data.fields?.[0]?.code;
  switch (code) {
    case 'additional_flow_business_license_required':
      return m.mfa_flow_error_additional_flow_business_license();
    case 'business_license_required':
      return m.mfa_flow_error_business_license();
    case 'smtp_not_configured':
      return m.mfa_flow_error_smtp_not_configured();
    case 'oidc_provider_missing':
      return m.mfa_flow_error_oidc_provider_missing();
    default:
      return m.mfa_flow_save_failed();
  }
};

type Props = {
  flow?: MfaFlowDetailResponse;
};

export const MfaFormPage = ({ flow }: Props) => {
  const navigate = useNavigate();
  const isEdit = flow !== undefined;
  const { mutateAsync: createMfaFlow } = useMutation({
    mutationFn: api.mfaFlow.create,
    meta: {
      invalidate: ['mfa-flow'],
    },
    onError: (error: AxiosError<MfaFlowErrorResponse>) => {
      Snackbar.error(getSaveErrorMessage(error));
    },
  });
  const { mutateAsync: updateMfaFlow } = useMutation({
    mutationFn: (request: UpdateMfaFlowRequest) => {
      if (!flow) throw new Error('Cannot update an MFA flow without an ID.');
      return api.mfaFlow.update(flow.id, request);
    },
    meta: {
      invalidate: ['mfa-flow'],
    },
    onError: (error: AxiosError<MfaFlowErrorResponse>) => {
      Snackbar.error(getSaveErrorMessage(error));
    },
  });
  const defaultValues = useMemo(
    () =>
      flow
        ? {
            title: flow.title,
            steps: flow.steps.map(({ id, methods }) => ({ id, methods })),
          }
        : {
            title: '',
            steps: [] as MfaConfigurationStepData[],
          },
    [flow],
  );
  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onChange: formSchema,
      onSubmit: formSchema,
    },
    onSubmit: async ({ value }) => {
      try {
        if (isEdit) {
          const request: UpdateMfaFlowRequest = {
            title: value.title,
            steps: value.steps.map(({ id, methods }) => ({
              ...(typeof id === 'number' ? { id } : {}),
              methods,
            })),
          };
          await updateMfaFlow(request);
        } else {
          const request: CreateMfaFlowRequest = {
            title: value.title,
            steps: value.steps.map(({ methods }) => ({ methods })),
          };
          await createMfaFlow(request);
        }
      } catch {
        return;
      }

      Snackbar.default(isEdit ? m.mfa_flow_updated() : m.mfa_flow_created());
      await navigate({ to: '/mfa' });
    },
  });

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        event.stopPropagation();
        void form.handleSubmit();
      }}
    >
      <EditPage
        pageTitle={m.cmp_nav_item_mfa()}
        links={[
          <Link key="mfa-flows" to="/mfa">
            {m.cmp_nav_item_mfa()}
          </Link>,
          isEdit ? (
            <Link
              key="edit-mfa-flow"
              to="/mfa-flow/$id/edit"
              params={{ id: `${flow.id}` }}
            >
              {m.mfa_flow_breadcrumb_edit()}
            </Link>
          ) : (
            <Link key="create-mfa-flow" to="/mfa/add-flow">
              {m.mfa_flow_breadcrumb_create()}
            </Link>
          ),
        ]}
        onBack={() => navigate({ to: '/mfa' })}
        headerProps={{
          icon: 'activity-notes',
          title: isEdit ? m.mfa_flow_form_title_edit() : m.mfa_flow_form_title_create(),
          subtitle: m.mfa_flow_form_subtitle(),
        }}
      >
        <form.AppForm>
          <MarkedSection icon="settings">
            <MarkedSectionHeader title={m.mfa_flow_form_general_settings()} />
            <form.AppField name="title">
              {(field) => <field.FormInput required label={m.mfa_flow_form_name()} />}
            </form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <MarkedSection icon="access-settings">
            <MarkedSectionHeader
              title={m.mfa_flow_form_methods_title()}
              description={m.mfa_flow_form_methods_description()}
            />
            <form.AppField name="steps">{() => <FormMfaConfiguration />}</form.AppField>
          </MarkedSection>
          <Divider spacing={ThemeSpacing.Xl2} />
          <Controls>
            <div className="right">
              <Button
                type="button"
                variant="secondary"
                text={m.controls_cancel()}
                onClick={() => {
                  void navigate({ to: '/mfa' });
                }}
              />
              <form.FormSubmitButton
                text={
                  isEdit ? m.controls_save_changes() : m.mfa_flow_form_action_create()
                }
              />
            </div>
          </Controls>
        </form.AppForm>
      </EditPage>
    </form>
  );
};

const FormMfaConfiguration = () => {
  const field = useFieldContext<MfaConfigurationStepData[]>();
  const error = useFormFieldError();

  return (
    <MfaConfiguration
      steps={field.state.value}
      onChange={field.handleChange}
      error={error}
    />
  );
};
