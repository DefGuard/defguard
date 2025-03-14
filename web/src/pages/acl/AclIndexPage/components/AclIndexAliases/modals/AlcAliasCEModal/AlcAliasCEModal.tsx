import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { omit } from 'lodash-es';
import { useEffect, useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { FormSelect } from '../../../../../../../shared/defguard-ui/components/Form/FormSelect/FormSelect';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import useApi from '../../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../../shared/queries';
import { protocolOptions, protocolToString } from '../../../../../utils';
import { aclPortsValidator } from '../../../../../validators';
import { useAclAliasCEModal } from './store';

export const AlcAliasCEModal = () => {
  const isOpen = useAclAliasCEModal((s) => s.visible);

  const [close, reset] = useAclAliasCEModal((s) => [s.close, s.reset], shallow);

  useEffect(() => {
    return () => {
      reset();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ModalWithTitle
      id="acl-alias-ce-modal"
      title="Create Alias"
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
  const queryClient = useQueryClient();
  const closeModal = useAclAliasCEModal((s) => s.close, shallow);
  const initialAlias = useAclAliasCEModal((s) => s.alias);
  const isEditMode = isPresent(initialAlias);
  const toaster = useToaster();

  const { LL } = useI18nContext();
  const {
    acl: {
      aliases: { createAlias, editAlias },
    },
  } = useApi();

  const schema = useMemo(
    () =>
      z.object({
        name: z.string(),
        ports: aclPortsValidator(LL),
        destination: z.string(),
        protocols: z.number().array(),
      }),
    [LL],
  );

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useMemo((): FormFields => {
    let defaultValues: FormFields;
    if (isPresent(initialAlias)) {
      defaultValues = omit(initialAlias, ['id']);
    } else {
      defaultValues = {
        destination: '',
        name: '',
        ports: '',
        protocols: [],
      };
    }
    return defaultValues;
  }, [initialAlias]);

  const {
    handleSubmit,
    control,
    formState: { isSubmitting },
  } = useForm<FormFields>({
    mode: 'all',
    resolver: zodResolver(schema),
    defaultValues,
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    console.log(values);
    try {
      if (isEditMode) {
        await editAlias({
          ...values,
          id: initialAlias.id,
        });
        toaster.success('Alias modified');
      } else {
        await createAlias(values);
        toaster.success('Alias created');
      }
      await queryClient.invalidateQueries({
        queryKey: [QueryKeys.FETCH_ACL_ALIASES],
      });
      closeModal();
    } catch (e) {
      toaster.error(LL.messages.error());
      console.error(e);
    }
    closeModal();
  };

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput controller={{ control, name: 'name' }} label="Alias Name" />
      <div className="header">
        <h2>Destination</h2>
      </div>
      <FormInput
        controller={{ control, name: 'destination' }}
        label="IPv4/6 CIDR range or address"
      />
      <FormInput controller={{ control, name: 'ports' }} label="Port or Port Range" />
      <FormSelect
        controller={{ control, name: 'protocols' }}
        label="Protocols"
        placeholder="All Protocols"
        options={protocolOptions}
        searchable={false}
        renderSelected={(val) => ({ displayValue: protocolToString(val), key: val })}
        disposable
      />
      <div className="controls">
        <Button
          className="cancel"
          text="Cancel"
          onClick={() => {
            closeModal();
          }}
          size={ButtonSize.LARGE}
          disabled={isSubmitting}
        />
        <Button
          className="submit"
          text="Create Alias"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          type="submit"
          loading={isSubmitting}
        />
      </div>
    </form>
  );
};
