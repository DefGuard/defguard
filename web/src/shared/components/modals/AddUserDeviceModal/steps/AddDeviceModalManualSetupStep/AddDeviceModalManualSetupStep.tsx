import z from 'zod';
import { m } from '../../../../../../paraglide/messages';
import { Button } from '../../../../../defguard-ui/components/Button/Button';
import { ModalControls } from '../../../../../defguard-ui/components/ModalControls/ModalControls';
import { useAppForm, withForm } from '../../../../../form';
import { formChangeLogic } from '../../../../../formLogic';
import './style.scss';
import { useStore } from '@tanstack/react-form';
import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import api from '../../../../../api/api';
import { SizedBox } from '../../../../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../defguard-ui/types';
import { patternValidWireguardKey } from '../../../../../patterns';
import { generateWGKeys } from '../../../../../utils/generateWGKeys';
import { useAddUserDeviceModal } from '../../store/useAddUserDeviceModal';
import { AddUserDeviceModalStep } from '../../types';

const publicKeySchema = z
  .string()
  .trim()
  .min(1, m.form_error_required())
  .length(44, m.form_error_invalid())
  .regex(patternValidWireguardKey, m.form_error_invalid());

const getFormSchema = (deviceNames: string[]) =>
  z
    .object({
      name: z
        .string()
        .trim()
        .min(1, m.form_error_required())
        .refine((value) => !deviceNames.includes(value), m.form_error_name_reserved()),
      genChoice: z.enum(['manual', 'auto']),
      publicKey: z.string().optional(),
    })
    .superRefine((value, ctx) => {
      if (value.genChoice === 'manual') {
        const result = publicKeySchema.safeParse(value.publicKey);
        if (!result.success) {
          ctx.addIssue({
            code: 'custom',
            message: result.error.message,
            continue: false,
            path: ['publicKey'],
          });
        }
      }
    });

type FormFields = z.infer<ReturnType<typeof getFormSchema>>;

const defaultValues: FormFields = {
  name: '',
  genChoice: 'auto',
  publicKey: '',
};

export const AddDeviceModalManualSetupStep = () => {
  const devices = useAddUserDeviceModal((s) => s.devices);
  const username = useAddUserDeviceModal((s) => s.user?.username as string);

  const { mutateAsync: createDevice } = useMutation({
    mutationFn: api.device.addDevice,
    meta: {
      invalidate: [['user-overview'], ['user']],
    },
  });

  const formSchema = useMemo(
    () => getFormSchema((devices ?? []).map((d) => d.name)),
    [devices],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      let publicKey: string;
      let privateKey: string | undefined;

      if (value.genChoice === 'auto') {
        const keys = generateWGKeys();
        publicKey = keys.publicKey;
        privateKey = keys.privateKey;
      } else {
        publicKey = value.publicKey as string;
      }

      const createResponse = await createDevice({
        name: value.name,
        username,
        wireguard_pubkey: publicKey,
      });

      if (!createResponse.data.configs.length) {
        useAddUserDeviceModal.getState().close();
      }

      useAddUserDeviceModal.setState({
        step: AddUserDeviceModalStep.ManualConfiguration,
        manualConfig: {
          publicKey,
          privateKey,
        },
        createDeviceResponse: createResponse.data,
      });
    },
  });

  return (
    <div id="add-user-device-manual-setup">
      <header>
        <p>{m.modal_add_user_device_manual_setup_title()}</p>
        <p>{m.modal_add_user_device_manual_setup_explain()}</p>
      </header>
      <SizedBox height={ThemeSpacing.Xl2} />
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="name">
            {(field) => <field.FormInput label={m.form_label_device_name()} required />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl3} />
          <div className="choice">
            <p>{m.modal_add_user_device_manual_setup_choice()}</p>
            <form.AppField name="genChoice">
              {(field) => (
                <field.FormRadio
                  value="auto"
                  text={m.modal_add_user_device_manual_setup_choice_auto()}
                />
              )}
            </form.AppField>
            <form.AppField name="genChoice">
              {(field) => (
                <field.FormRadio
                  value="manual"
                  text={m.modal_add_user_device_manual_setup_choice_manual()}
                />
              )}
            </form.AppField>
          </div>
          <PublicKeyField form={form} />
        </form.AppForm>
      </form>
      <ModalControls
        cancelProps={{
          testId: 'cancel',
          text: m.controls_cancel(),
          disabled: form.state.isSubmitting,
          onClick: () => {
            useAddUserDeviceModal.getState().close();
          },
        }}
        submitProps={{
          testId: 'continue',
          text: m.controls_continue(),
          loading: form.state.isSubmitting,
          onClick: () => {
            form.handleSubmit();
          },
        }}
      >
        <Button
          disabled={form.state.isSubmitting}
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddUserDeviceModal.setState({
              step: AddUserDeviceModalStep.StartChoice,
            });
          }}
        />
      </ModalControls>
    </div>
  );
};

const PublicKeyField = withForm({
  defaultValues,
  render: ({ form }) => {
    const choice = useStore(form.store, (s) => s.values.genChoice);

    if (choice === 'auto') return null;
    return (
      <>
        <SizedBox height={ThemeSpacing.Xl3} />
        <form.AppField name="publicKey">
          {(field) => <field.FormInput label={m.form_label_public_key()} required />}
        </form.AppField>
      </>
    );
  },
});
