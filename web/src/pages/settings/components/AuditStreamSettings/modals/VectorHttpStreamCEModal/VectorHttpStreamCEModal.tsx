import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { useCallback, useEffect, useMemo } from 'react';
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
import queryClient from '../../../../../../shared/query-client';
import { removeEmptyStrings } from '../../../../../../shared/utils/removeEmptyStrings';
import { trimObjectStrings } from '../../../../../../shared/utils/trimObjectStrings';
import { useVectorHttpStreamCEModal } from './store';

export const VectorHttpStreamCEModal = () => {
  const isOpen = useVectorHttpStreamCEModal((s) => s.visible);
  const [close, reset] = useVectorHttpStreamCEModal((s) => [s.close, s.reset], shallow);

  useEffect(() => {
    return () => {
      reset();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModalWithTitle
      includeDefaultStyles
      title="Add Vector (http) stream"
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
  const closeModal = useVectorHttpStreamCEModal((s) => s.close, shallow);
  const [isEdit, initialData] = useVectorHttpStreamCEModal((s) => [
    s.edit,
    s.initStreamData,
  ]);

  const { LL } = useI18nContext();

  const {
    auditStream: { createAuditStream, modifyAuditStream },
  } = useApi();

  const schema = useMemo(
    () =>
      z.object({
        name: z.string(),
        url: z.string().min(1, LL.form.error.required()).url(),
        username: z.string(),
        password: z.string(),
        cert: z.string(),
      }),
    [LL.form.error],
  );

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useMemo((): FormFields => {
    if (isEdit && isPresent(initialData)) {
      return {
        name: initialData.name ?? '',
        url: initialData.config.url,
        username: initialData.config.username ?? '',
        password: initialData.config.password ?? '',
        cert: initialData.config.cert ?? '',
      };
    }
    return {
      name: '',
      password: '',
      url: '',
      username: '',
      cert: '',
    };
  }, [initialData, isEdit]);

  const { handleSubmit, control } = useForm({
    defaultValues,
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const handleSuccess = useCallback(() => {
    closeModal();
    void queryClient.invalidateQueries({
      queryKey: ['audit_stream'],
    });
  }, [closeModal]);

  const { mutateAsync: modifyMutation } = useMutation({
    mutationFn: modifyAuditStream,
    onSuccess: () => {
      handleSuccess();
    },
  });

  const { mutateAsync: createMutation } = useMutation({
    mutationFn: createAuditStream,
    onSuccess: () => {
      handleSuccess();
    },
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    // prepare output
    const { name, ...config } = removeEmptyStrings(trimObjectStrings(values));

    if (isPresent(initialData)) {
      await modifyMutation({
        id: initialData.id,
        name,
        stream_type: 'vector_http',
        stream_config: config,
      });
    } else {
      await createMutation({
        name,
        stream_config: config,
        stream_type: 'vector_http',
      });
    }
  };

  return (
    <>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput controller={{ control, name: 'name' }} label="Name" />
        <FormInput controller={{ control, name: 'url' }} required label="Url" />
        <FormInput controller={{ control, name: 'username' }} label="Username" />
        <FormInput
          controller={{ control, name: 'password' }}
          type="password"
          label="Password"
        />
        <FormInput controller={{ control, name: 'cert' }} label="Certificate" />
        <div className="controls">
          <Button
            text={LL.common.controls.cancel()}
            className="cancel"
            onClick={() => {
              closeModal();
            }}
          />
          <Button
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={LL.common.controls.submit()}
            className="submit"
            type="submit"
          />
        </div>
      </form>
    </>
  );
};
