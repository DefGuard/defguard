import { useMutation } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import type { SelectOption } from '../../../shared/defguard-ui/components/Select/types';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { CAOption, type CAOptionType, SetupPageStep } from '../types';
import { useSetupWizardStore } from '../useSetupWizardStore';
import './style.scss';
import { BadgeVariant } from '../../../shared/defguard-ui/components/Badge/types';

type ValidityValue = 1 | 2 | 3 | 5 | 10;

const validityOptions: SelectOption<ValidityValue>[] = [
  { key: 1, label: m.initial_setup_ca_validity_one_year(), value: 1 },
  { key: 2, label: m.initial_setup_ca_validity_years({ years: 2 }), value: 2 },
  { key: 3, label: m.initial_setup_ca_validity_years({ years: 3 }), value: 3 },
  { key: 5, label: m.initial_setup_ca_validity_years({ years: 5 }), value: 5 },
  {
    key: 10,
    label: m.initial_setup_ca_validity_years({ years: 10 }),
    value: 10,
  },
];

type CreateCAFormFields = CreateCAStoreValues;

type CreateCAStoreValues = {
  ca_common_name: string;
  ca_email: string;
  ca_validity_period_years: number;
};

type UploadCAFormFields = UploadCAStoreValues;

type UploadCAStoreValues = {
  ca_cert_file: File | null;
};

const readFileAsText = (file: File): Promise<string> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsText(file);
  });
};

export const SetupCertificateAuthorityStep = () => {
  const setActiveStep = useSetupWizardStore((s) => s.setActiveStep);
  const caOption = useSetupWizardStore((s) => s.ca_option);
  const setCAOption = useCallback((option: CAOptionType) => {
    useSetupWizardStore.setState({ ca_option: option });
  }, []);

  const createCAdefaultValues = useSetupWizardStore(
    useShallow(
      (s): CreateCAFormFields => ({
        ca_common_name: s.ca_common_name,
        ca_email: s.ca_email,
        ca_validity_period_years: s.ca_validity_period_years,
      }),
    ),
  );

  const uploadCAdefaultValues: UploadCAFormFields = {
    ca_cert_file: undefined as unknown as File,
  };

  const createFormSchema = useMemo(
    () =>
      z.object({
        ca_common_name: z
          .string()
          .min(1, m.initial_setup_ca_error_common_name_required()),
        ca_email: z
          .email(m.initial_setup_ca_error_email_invalid())
          .min(1, m.initial_setup_ca_error_email_required()),
        ca_validity_period_years: z
          .number()
          .min(1, m.initial_setup_ca_error_validity_min()),
      }),
    [],
  );

  const uploadFormSchema = useMemo(
    () =>
      z.object({
        ca_cert_file: z
          .file()
          .refine((file) => isPresent(file), m.initial_setup_ca_error_cert_required()),
      }),
    [],
  );

  const { mutate: createCA, isPending: isCreatingCA } = useMutation({
    mutationFn: api.initial_setup.createCA,
    onSuccess: () => {
      setActiveStep(SetupPageStep.CASummary);
    },
    onError: (error) => {
      console.error('Failed to create CA:', error);
      Snackbar.error(m.initial_setup_ca_error_create_failed());
    },
    meta: {
      invalidate: ['initial_setup', 'ca'],
    },
  });

  const { mutate: uploadCA, isPending: isUploadingCA } = useMutation({
    mutationFn: api.initial_setup.uploadCA,
    onSuccess: () => {
      setActiveStep(SetupPageStep.CASummary);
    },
    onError: (error) => {
      console.error('Failed to upload CA:', error);
      Snackbar.error(m.initial_setup_ca_error_upload_failed());
    },
    meta: {
      invalidate: ['initial_setup', 'ca'],
    },
  });

  const createForm = useAppForm({
    defaultValues: createCAdefaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: createFormSchema,
      onChange: createFormSchema,
    },
    onSubmit: ({ value }) => {
      useSetupWizardStore.setState({
        ca_common_name: value.ca_common_name,
        ca_email: value.ca_email,
        ca_validity_period_years: value.ca_validity_period_years,
      });
      createCA({
        common_name: value.ca_common_name,
        email: value.ca_email,
        validity_period_years: value.ca_validity_period_years,
      });
    },
  });

  const uploadForm = useAppForm({
    defaultValues: uploadCAdefaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: uploadFormSchema,
      onChange: uploadFormSchema,
    },
    onSubmit: async ({ value }) => {
      if (!value.ca_cert_file) return;
      const certContent = await readFileAsText(value.ca_cert_file);
      uploadCA({ cert_file: certContent });
    },
  });

  const CreateCAForm = () => {
    const form = createForm;
    return (
      <div className="ca-settings">
        <form
          onSubmit={(e) => {
            e.stopPropagation();
            e.preventDefault();
            form.handleSubmit();
          }}
        >
          <form.AppForm>
            <form.AppField name="ca_common_name">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_ca_label_common_name()}
                  type="text"
                  placeholder={m.initial_setup_ca_placeholder_common_name()}
                />
              )}
            </form.AppField>

            <form.AppField name="ca_email">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_ca_label_email()}
                  placeholder={m.initial_setup_ca_placeholder_email()}
                />
              )}
            </form.AppField>

            <form.AppField name="ca_validity_period_years">
              {(field) => (
                <field.FormSelect
                  required
                  label={m.initial_setup_ca_label_validity()}
                  options={validityOptions}
                />
              )}
            </form.AppField>
          </form.AppForm>
        </form>
      </div>
    );
  };

  // const UploadCAForm = () => {
  // 	const form = uploadForm;
  // 	return (
  // 		<form
  // 			onSubmit={(e) => {
  // 				e.stopPropagation();
  // 				e.preventDefault();
  // 				form.handleSubmit();
  // 			}}
  // 		>
  // 			<form.AppForm>
  // 				<form.AppField name="ca_cert_file">
  // 					{(field) => <field.FormUploadField />}
  // 				</form.AppField>
  // 				<SizedBox height={ThemeSpacing.Xl} />
  // 			</form.AppForm>
  // 		</form>
  // 	);
  // };

  const handleBack = () => {
    setActiveStep(SetupPageStep.GeneralConfig);
  };

  const handleNext = () => {
    if (caOption === CAOption.Create) {
      createForm.handleSubmit();
    } else if (caOption === CAOption.UseOwn) {
      uploadForm.handleSubmit();
    }
  };

  const isPending = isCreatingCA || isUploadingCA;

  return (
    <WizardCard>
      <InteractiveBlock
        title={m.initial_setup_ca_option_create_title()}
        value={caOption === CAOption.Create}
        onClick={() => setCAOption(CAOption.Create)}
        content={m.initial_setup_ca_option_create_description()}
        badge={{
          text: m.misc_recommended(),
          variant: BadgeVariant.Success,
        }}
      >
        <SizedBox height={ThemeSpacing.Xl2} />
        {caOption === CAOption.Create && <CreateCAForm />}
      </InteractiveBlock>

      {/* Temporarily disabled */}
      {/* <SizedBox height={ThemeSpacing.Xl3} />

      <InteractiveBlock
        title="Use your own certificate authority"
        value={caOption === CAOption.UseOwn}
        onClick={() => setCAOption(CAOption.UseOwn)}
        content="Upload your certificate authority certificate and Defguard will use it to issue and configure certificates for components."
        >        <SizedBox height={ThemeSpacing.Xl} />
          {caOption === CAOption.UseOwn && <UploadCAForm />}
        </InteractiveBlock> */}

      <ModalControls
        cancelProps={{
          text: m.controls_back(),
          onClick: handleBack,
          disabled: isPending,
          variant: 'outlined',
        }}
        submitProps={{
          text: m.controls_continue(),
          onClick: handleNext,
          loading: isPending,
          disabled: isPending || !isPresent(caOption),
        }}
      />
    </WizardCard>
  );
};
