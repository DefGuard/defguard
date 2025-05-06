import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormCheckBox } from '../../../../shared/defguard-ui/components/Form/FormCheckBox/FormCheckBox';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { useModalStore } from '../../../../shared/hooks/store/useModalStore';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../shared/mutations';
import { QueryKeys } from '../../../../shared/queries';

export const WebhookForm = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    webhook: { addWebhook, editWebhook },
  } = useApi();
  const modalState = useModalStore((state) => state.webhookModal);
  const setModalState = useModalStore((state) => state.setWebhookModal);
  const editMode = useMemo(() => !isUndefined(modalState.webhook), [modalState.webhook]);

  const queryClient = useQueryClient();

  const zodSchema = useMemo(
    () =>
      z
        .object({
          url: z.string().min(1, LL.modals.webhookModal.form.error.urlRequired()),
          description: z
            .string()
            .min(1, LL.form.error.required())
            .min(4, LL.form.error.minimumLength())
            .max(65, LL.form.error.maximumLength()),
          token: z
            .string()
            .min(1, LL.form.error.required())
            .min(3, LL.form.error.minimumLength())
            .max(250, LL.form.error.maximumLength()),
          enabled: z.boolean(),
          on_user_created: z.boolean(),
          on_user_deleted: z.boolean(),
          on_user_modified: z.boolean(),
          on_hwkey_provision: z.boolean(),
        })
        .superRefine((val, ctx) => {
          if (val.enabled) {
            if (
              !val.on_hwkey_provision &&
              !val.on_user_created &&
              !val.on_user_deleted &&
              !val.on_user_modified
            ) {
              ctx.addIssue({
                code: 'custom',
                message: 'At least one event needs to be present',
              });
            }
          }
        }),
    [LL.form.error, LL.modals.webhookModal.form.error],
  );

  type FormFields = z.infer<typeof zodSchema>;

  const defaultFormState = useMemo((): FormFields => {
    if (!isUndefined(modalState.webhook)) {
      return modalState.webhook;
    }
    const defaultValues: FormFields = {
      url: '',
      description: '',
      token: '',
      enabled: true,
      on_hwkey_provision: false,
      on_user_created: false,
      on_user_deleted: false,
      on_user_modified: false,
    };
    return defaultValues;
  }, [modalState.webhook]);

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues: defaultFormState,
    mode: 'all',
    resolver: zodResolver(zodSchema),
  });

  const { mutate: addWebhookMutation, isPending: addWebhookIsLoading } = useMutation({
    mutationKey: [MutationKeys.EDIT_WEBHOOK],
    mutationFn: addWebhook,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_WEBHOOKS],
      });
      toaster.success(LL.modals.webhookModal.form.messages.successAdd());
      setModalState({ visible: false, webhook: undefined });
    },
    onError: (err) => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_WEBHOOKS],
      });
      toaster.error(LL.messages.error());
      setModalState({ visible: false, webhook: undefined });
      console.error(err);
    },
  });

  const { mutate: editWebhookMutation, isPending: editMutationIsLoading } = useMutation({
    mutationKey: [MutationKeys.EDIT_WEBHOOK],
    mutationFn: editWebhook,
    onSuccess: () => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_WEBHOOKS],
      });
      toaster.success(LL.modals.webhookModal.form.messages.successModify());
      setModalState({ visible: false, webhook: undefined });
    },
    onError: (err) => {
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_WEBHOOKS],
      });
      toaster.error(LL.messages.error());
      setModalState({ visible: false, webhook: undefined });
      console.error(err);
    },
  });

  const onValidSubmit: SubmitHandler<FormFields> = (values) => {
    if (editMode) {
      if (modalState.webhook) {
        editWebhookMutation({ ...modalState.webhook, ...values });
      }
    } else {
      addWebhookMutation({ ...values, enabled: true });
    }
  };

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormCheckBox
        label={LL.common.controls.enabled()}
        labelPlacement="right"
        controller={{ control, name: 'enabled' }}
      />
      <FormInput
        label={LL.modals.webhookModal.form.fields.url.label()}
        controller={{ control, name: 'url' }}
        placeholder={LL.modals.webhookModal.form.fields.url.placeholder()}
        required
      />
      <FormInput
        label={LL.modals.webhookModal.form.fields.description.label()}
        controller={{ control, name: 'description' }}
        placeholder={LL.modals.webhookModal.form.fields.description.placeholder()}
        required
        type="text"
      />
      <FormInput
        label={LL.modals.webhookModal.form.fields.token.label()}
        controller={{ control, name: 'token' }}
        placeholder={LL.modals.webhookModal.form.fields.token.placeholder()}
        required
      />
      <h3>{LL.modals.webhookModal.form.triggers()}</h3>
      <div className="events">
        <FormCheckBox
          controller={{ control, name: 'on_user_created' }}
          label={LL.modals.webhookModal.form.fields.userCreated.label()}
          labelPlacement="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_deleted' }}
          label={LL.modals.webhookModal.form.fields.userDeleted.label()}
          labelPlacement="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_user_modified' }}
          label={LL.modals.webhookModal.form.fields.userModified.label()}
          labelPlacement="right"
        />
        <FormCheckBox
          controller={{ control, name: 'on_hwkey_provision' }}
          label={LL.modals.webhookModal.form.fields.hwkeyProvision.label()}
          labelPlacement="right"
        />
      </div>
      <div className="controls">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          size={ButtonSize.LARGE}
          type="button"
          text={LL.form.cancel()}
          className="cancel"
          onClick={() => setModalState({ visible: false, webhook: undefined })}
        />
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          size={ButtonSize.LARGE}
          type="submit"
          className="submit"
          text={LL.form.submit()}
          loading={addWebhookIsLoading || editMutationIsLoading}
        />
      </div>
    </form>
  );
};
