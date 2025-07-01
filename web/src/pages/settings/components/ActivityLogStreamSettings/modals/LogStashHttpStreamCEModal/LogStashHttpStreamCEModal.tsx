import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { AxiosError } from 'axios';
import { useCallback, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { isPresent } from '../../../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import queryClient from '../../../../../../shared/query-client';
import {
  ActivityLogStreamLogstashHttp,
  ActivityLogStreamType,
} from '../../../../../../shared/types';
import { removeEmptyStrings } from '../../../../../../shared/utils/removeEmptyStrings';
import { trimObjectStrings } from '../../../../../../shared/utils/trimObjectStrings';
import { activityLogStreamTypeToLabel } from '../../utils/activityLogStreamToLabel';
import { useLogstashHttpStreamCEModalStore } from './store';

export const LogStashHttpStreamCEModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.activityLogStreamSettings.modals.logstash;

  const [close, reset] = useLogstashHttpStreamCEModalStore(
    (s) => [s.close, s.reset],
    shallow,
  );
  const [isOpen, isEdit] = useLogstashHttpStreamCEModalStore((s) => [
    s.visible,
    isPresent(s.initStreamData),
  ]);

  const title = isEdit ? localLL.modify() : localLL.create();

  return (
    <ModalWithTitle
      title={title}
      isOpen={isOpen}
      onClose={() => {
        close();
      }}
      afterClose={() => {
        reset();
      }}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.activityLogStreamSettings;
  const formLabels = LL.settingsPage.activityLogStreamSettings.modals.shared.formLabels;
  const {
    activityLogStream: { createActivityLogStream, modifyActivityLogStream },
  } = useApi();
  const close = useLogstashHttpStreamCEModalStore((s) => s.close, shallow);
  const initialData = useLogstashHttpStreamCEModalStore((s) => s.initStreamData);
  const toaster = useToaster();

  const onError = useCallback(
    (e: AxiosError) => {
      toaster.error(LL.messages.error());
      console.error(e);
    },
    [LL.messages, toaster],
  );

  const { mutateAsync: createStreamMutation } = useMutation({
    mutationFn: createActivityLogStream,
    onSuccess: () => {
      toaster.success(
        localLL.messages.destinationCrud.create({
          destination: activityLogStreamTypeToLabel('logstash_http'),
        }),
      );
      void queryClient.invalidateQueries({
        queryKey: ['activity_stream'],
      });
      close();
    },
    onError,
  });

  const { mutateAsync: modifyStreamMutation } = useMutation({
    mutationFn: modifyActivityLogStream,
    onSuccess: () => {
      toaster.success(
        localLL.messages.destinationCrud.modify({
          destination: activityLogStreamTypeToLabel('logstash_http'),
        }),
      );
      void queryClient.invalidateQueries({
        queryKey: ['activity_stream'],
      });
      close();
    },
    onError,
  });

  const isEdit = isPresent(initialData);

  const schema = useMemo(
    () =>
      z.object({
        name: z.string().min(1, LL.form.error.required()),
        url: z.string().min(1, LL.form.error.required()).url(LL.form.error.urlInvalid()),
        username: z.string(),
        password: z.string(),
        cert: z.string(),
      }),
    [LL.form.error],
  );

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useMemo((): FormFields => {
    if (isPresent(initialData)) {
      const { name, config } = initialData;
      const { cert, url, password, username } = config;
      return {
        name: name,
        cert: cert ?? '',
        password: password ?? '',
        username: username ?? '',
        url,
      };
    }

    return {
      cert: '',
      name: '',
      url: '',
      password: '',
      username: '',
    };
  }, [initialData]);

  const {
    handleSubmit,
    control,
    resetField,
    formState: { isSubmitting },
  } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    const { name, ...config } = removeEmptyStrings(trimObjectStrings(values));
    const streamType: ActivityLogStreamType = 'logstash_http';

    const logstashConfig: ActivityLogStreamLogstashHttp = config;

    if (isEdit) {
      await modifyStreamMutation({
        id: initialData.id,
        stream_type: streamType,
        stream_config: logstashConfig,
        name,
      });
    } else {
      await createStreamMutation({
        stream_type: streamType,
        stream_config: logstashConfig,
        name,
      });
    }
  };

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput
        label={formLabels.name()}
        controller={{ control, name: 'name' }}
        required
      />
      <FormInput
        label={formLabels.url()}
        controller={{ control, name: 'url' }}
        required
      />
      <FormInput
        controller={{ control, name: 'username' }}
        label={formLabels.username()}
        disposeHandler={() => {
          resetField('username', { defaultValue: '' });
        }}
      />
      <FormInput
        controller={{ control, name: 'password' }}
        type="password"
        label={formLabels.password()}
        disposeHandler={() => {
          resetField('password', { defaultValue: '' });
        }}
      />
      <FormInput
        label={formLabels.cert()}
        controller={{ control, name: 'cert' }}
        disposable
        disposeHandler={() => {
          resetField('cert', { defaultValue: '' });
        }}
      />

      <div className="controls">
        <Button
          text={LL.common.controls.cancel()}
          disabled={isSubmitting}
          onClick={() => {
            close();
          }}
        />
        <Button
          text={LL.common.controls.submit()}
          styleVariant={ButtonStyleVariant.PRIMARY}
          type="submit"
          loading={isSubmitting}
        />
      </div>
    </form>
  );
};
