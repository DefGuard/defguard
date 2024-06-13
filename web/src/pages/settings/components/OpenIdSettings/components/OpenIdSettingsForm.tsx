// import { zodResolver } from '@hookform/resolvers/zod';
// import { useMemo, useRef } from 'react';
// import { SubmitHandler, useForm } from 'react-hook-form';
// import { z } from 'zod';

import { useQuery } from '@tanstack/react-query';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import IconCheckmarkWhite from '../../../../../shared/components/svg/IconCheckmarkWhite';
// import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../shared/hooks/useApi';
// import { useToaster } from '../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../shared/queries';
// import { useSettingsPage } from '../../../hooks/useSettingsPage';
import { ProviderDetails } from './ProviderDetails';

export const OpenIdSettingsForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.openIdSettings;
  // const submitRef = useRef<HTMLInputElement | null>(null);
  // const settings = useSettingsPage((state) => state.settings);
  // const {
  //   settings: { patchSettings },
  // } = useApi();

  // const queryClient = useQueryClient();

  const {
    settings: { fetchOpenIdProviders },
  } = useApi();
  const { data: providers, isLoading } = useQuery({
    queryFn: fetchOpenIdProviders,
    queryKey: [QueryKeys.FETCH_OPENID_PROVIDERS],
    refetchOnMount: true,
    refetchOnWindowFocus: false,
  });

  console.log('providers:', providers);

  // const toaster = useToaster();

  // const { isSaving, mutate } = useMutation({
  //   mutationFn: patchSettings,
  //   onSuccess: () => {
  //     queryClient.invalidateQueries([QueryKeys.FETCH_OPENID_PROVIDERS]);
  //     toaster.success(LL.settingsPage.messages.editSuccess());
  //   },
  // });

  // const defaultValues = useMemo(
  //   (): FormFields => ({
  //     name: settings?.name ?? '',
  //     document_url: settings?.document_url ?? '',
  //   }),
  //   [settings],
  // );

  // const { handleSubmit, control } = useForm<FormFields>({
  //   resolver: zodResolver(schema),
  //   defaultValues,
  //   mode: 'all',
  // });

  // const handleValidSubmit: SubmitHandler<FormFields> = (data) => {
  //   mutate(data);
  // };
  return (
    <section id="openid-settings">
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
      <>
        {providers && providers.length > 0 && (
          <div className="devices">
            {providers.map((provider) => (
              <ProviderDetails key={provider.id} provider={provider} />
            ))}
          </div>
        )}
      </>
      {/* <form id="openid-settings-form" onSubmit={handleSubmit(handleValidSubmit)}> */}
      {/*   <FormInput */}
      {/*     controller={{ control, name: 'name' }} */}
      {/*     label={localLL.form.labels.name()} */}
      {/*   /> */}
      {/*   <FormInput */}
      {/*     controller={{ control, name: 'document_url' }} */}
      {/*     label={localLL.form.labels.documentUrl()} */}
      {/*   /> */}
      {/*   <input type="submit" aria-hidden="true" className="hidden" ref={submitRef} /> */}
      {/* </form> */}
    </section>
  );
};
