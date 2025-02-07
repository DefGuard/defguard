import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import SvgIconCheckmark from '../../../../../../../shared/components/svg/IconCheckmark';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { ActionButton } from '../../../../../../../shared/defguard-ui/components/Layout/ActionButton/ActionButton';
import { ActionButtonVariant } from '../../../../../../../shared/defguard-ui/components/Layout/ActionButton/types';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ExpandableCard } from '../../../../../../../shared/defguard-ui/components/Layout/ExpandableCard/ExpandableCard';
import { MessageBox } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useClipboard } from '../../../../../../../shared/hooks/useClipboard';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../../shared/queries';
import { trimObjectStrings } from '../../../../../../../shared/utils/trimObjectStrings';
import { useAddApiTokenModal } from '../../useAddApiTokenModal';

type FormFields = {
  name: string;
};

const defaultValues: FormFields = {
  name: '',
};

export const AddApiTokenForm = () => {
  const { LL } = useI18nContext();
  const {
    user: { addApiToken },
  } = useApi({
    notifyError: true,
  });
  const toaster = useToaster();
  const localLL = LL.userPage.apiTokens.addModal;
  const closeModal = useAddApiTokenModal((s) => s.close);
  const user = useAddApiTokenModal((s) => s.user);
  const queryClient = useQueryClient();
  const { writeToClipboard } = useClipboard();

  const [generatedToken, setGeneratedToken] = useState<string | undefined>(undefined);

  const { mutate, isPending } = useMutation({
    mutationFn: addApiToken,
    onSuccess: (resp) => {
      toaster.success(LL.messages.success());
      void queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_API_TOKENS_INFO],
      });
      setGeneratedToken(resp.token);
    },
    onError: (e) => {
      console.error(e);
    },
  });

  const schema = useMemo(
    () =>
      z.object({
        name: z
          .string({
            required_error: LL.form.error.required(),
          })
          .min(1, LL.form.error.required())
          .min(4, LL.form.error.minimumLength()),
      }),
    [LL.form.error],
  );

  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    mode: 'all',
    defaultValues,
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    const trimmed = trimObjectStrings(values);
    if (user) {
      mutate({
        name: trimmed.name,
        username: user.username,
      });
    }
  };

  return generatedToken ? (
    <div className="copy-generated-token-content">
      <MessageBox
        type={MessageBoxType.WARNING}
        message={localLL.copyToken.warningMessage()}
      />

      <ExpandableCard
        title={localLL.copyToken.header()}
        actions={[
          <ActionButton
            variant={ActionButtonVariant.COPY}
            onClick={() => {
              void writeToClipboard(generatedToken);
            }}
            key={0}
          />,
        ]}
        expanded
        disableExpand
      >
        <p className="code">{generatedToken}</p>
      </ExpandableCard>
      <Button
        type="button"
        size={ButtonSize.SMALL}
        styleVariant={ButtonStyleVariant.STANDARD}
        text={LL.common.controls.close()}
        onClick={() => closeModal()}
      />
    </div>
  ) : (
    <form onSubmit={handleSubmit(handleValidSubmit)} id="add-api-token-modal-form">
      <FormInput
        controller={{ control, name: 'name' }}
        label={localLL.tokenForm.labels.name()}
        placeholder={localLL.tokenForm.placeholders.name()}
        autoComplete="off"
      />
      <div className="controls">
        <Button
          type="button"
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.cancel()}
          onClick={() => closeModal()}
        />
        <Button
          type="submit"
          icon={<SvgIconCheckmark />}
          size={ButtonSize.SMALL}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={localLL.tokenForm.submit()}
          loading={isPending}
        />
      </div>
    </form>
  );
};
