import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
import { SettingsOpenId } from '../../../../../shared/types';
import { useSettingsPage } from '../../../hooks/useSettingsPage';

type FormFields = SettingsOpenId;

export const OpenIdSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  const submitRef = useRef<HTMLInputElement | null>(null);
  const settings = useSettingsPage((state) => state.settings);
  const {
    settings: { patchSettings },
  } = useApi();

  const queryClient = useQueryClient();

  const toaster = useToaster();

  const { isLoading, mutate } = useMutation({
    mutationFn: patchSettings,
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toaster.success(LL.settingsPage.messages.editSuccess());
    },
  });

  const schema = useMemo(
    () =>
      z.object({
        name: z.string().min(1, LL.form.error.required()),
        document_url: z
          .string()
          .url(LL.form.error.invalid())
          .min(1, LL.form.error.required()),
      }),
    [LL.form.error],
  );

  const defaultValues = useMemo(
    (): FormFields => ({
      name: settings?.name ?? '',
      document_url: settings?.document_url ?? '',
    }),
    [settings],
  );

  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };
  return (
    <section id="oenid-settings">
      <header>
        <h2>{localLL.title()}</h2>
        <Button
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.SAVE}
          text={LL.common.controls.saveChanges()}
          type="submit"
          loading={isLoading}
          icon={<IconCheckmarkWhite />}
          onClick={() => submitRef.current?.click()}
        />
      </header>
      <form id="openid-settings-form" onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'name' }}
          label={localLL.form.labels.name()}
        />
        <FormInput
          controller={{ control, name: 'document_url' }}
          label={localLL.form.labels.documentUrl()}
        />
        <input type="submit" aria-hidden="true" className="hidden" ref={submitRef} />
      </form>
    </section>
  );
};
